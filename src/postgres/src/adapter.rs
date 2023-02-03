use crate::error::PostgresError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, Result, ResultExt};
use kyogre_core::{
    AisMigratorDestination, AisPosition, AisVesselMigrate, Arrival, ArrivalFilter, DataMessage,
    DateRange, DeliveryPoint, Departure, FileHashId, HashDiff, InsertError, NewAisPosition,
    NewAisStatic, NewArrival, NewDca, NewDeparture, NewLanding, NewTrip, QueryError,
    ScraperFileHashInboundPort, ScraperInboundPort, Trip, TripAssemblerConflict, TripAssemblerId,
    TripAssemblerInboundPort, TripAssemblerOutboundPort, TripCalculationTimer, TripDockPoints,
    TripPorts, TripPrecisionInboundPort, TripPrecisionOutboundPort, TripPrecisionUpdate,
    TripsConflictStrategy, UpdateError, Vessel, WebApiPort,
};
use orca_core::{PsqlLogStatements, PsqlSettings};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use std::collections::HashMap;
use tracing::{event, instrument, Level};

#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
}

enum AisProcessingAction {
    Exit,
    Continue,
    Retry {
        positions: Option<Vec<NewAisPosition>>,
        unique_static: Option<HashMap<i32, NewAisStatic>>,
    },
}

impl PostgresAdapter {
    pub async fn new(settings: &PsqlSettings) -> Result<PostgresAdapter, PostgresError> {
        let mut connections_per_pool = (settings.max_connections / 2) as u32;
        if connections_per_pool == 0 {
            connections_per_pool = 1;
        }

        let mut opts = PgConnectOptions::new()
            .username(&settings.username)
            .password(&settings.password)
            .host(&settings.ip)
            .port(settings.port as u16)
            .options([("plan_cache_mode", "force_custom_plan")]);

        if let Some(db_name) = &settings.db_name {
            opts = opts.database(db_name);
        }

        if let Some(root_cert_path) = &settings.root_cert {
            opts = opts
                .ssl_root_cert(root_cert_path)
                .ssl_mode(PgSslMode::VerifyFull);
        }

        match settings.log_statements {
            PsqlLogStatements::Enable => (),
            PsqlLogStatements::Disable => {
                opts.disable_statement_logging();
            }
        }

        let pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(20))
            .connect_with(opts)
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        Ok(PostgresAdapter { pool })
    }

    pub async fn do_migrations(&self) {
        sqlx::migrate!()
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .unwrap();
    }

    pub async fn consume_loop(
        self,
        mut receiver: tokio::sync::broadcast::Receiver<DataMessage>,
        process_confirmation: Option<tokio::sync::mpsc::Sender<()>>,
    ) {
        loop {
            let message = receiver.recv().await;
            let result = self.process_message(message).await;
            // Only enabled in tests
            if let Some(ref s) = process_confirmation {
                s.send(()).await.unwrap();
            }
            match result {
                AisProcessingAction::Exit => break,
                AisProcessingAction::Continue => (),
                AisProcessingAction::Retry {
                    positions,
                    unique_static,
                } => {
                    for _ in 0..2 {
                        self.insertion_retry(positions.as_deref(), unique_static.as_ref())
                            .await;
                    }
                }
            }
        }
    }

    #[instrument(skip_all, name = "postgres_insertion_retry")]
    async fn insertion_retry(
        &self,
        positions: Option<&[NewAisPosition]>,
        unique_static: Option<&HashMap<i32, NewAisStatic>>,
    ) {
        if let Some(positions) = positions {
            if let Err(e) = self.add_ais_positions(positions).await {
                event!(Level::ERROR, "failed to add ais positions: {:?}", e);
            }
        }

        if let Some(unique_static) = unique_static {
            if let Err(e) = self.add_ais_vessels(unique_static).await {
                event!(Level::ERROR, "failed to add ais static: {:?}", e);
            }
        }
    }

    #[instrument(skip_all, name = "postgres_insert_ais_data")]
    async fn process_message(
        &self,
        incoming: std::result::Result<DataMessage, tokio::sync::broadcast::error::RecvError>,
    ) -> AisProcessingAction {
        match incoming {
            Ok(message) => {
                let mut unique_static = HashMap::new();
                for v in message.static_messages {
                    unique_static.entry(v.mmsi).or_insert(v);
                }

                match (
                    self.add_ais_positions(&message.positions).await,
                    self.add_ais_vessels(&unique_static).await,
                ) {
                    (Ok(_), Ok(_)) => AisProcessingAction::Continue,
                    (Ok(_), Err(e)) => {
                        event!(Level::ERROR, "failed to add ais static: {:?}", e);
                        AisProcessingAction::Retry {
                            positions: None,
                            unique_static: Some(unique_static),
                        }
                    }
                    (Err(e), Ok(_)) => {
                        event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            unique_static: None,
                        }
                    }
                    (Err(e), Err(e2)) => {
                        event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                        event!(Level::ERROR, "failed to add ais static: {:?}", e2);
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            unique_static: Some(unique_static),
                        }
                    }
                }
            }
            Err(e) => match e {
                tokio::sync::broadcast::error::RecvError::Closed => {
                    event!(
                        Level::ERROR,
                        "sender half of ais broadcast channel closed unexpectedly, exiting"
                    );
                    AisProcessingAction::Exit
                }
                tokio::sync::broadcast::error::RecvError::Lagged(num_lagged) => {
                    event!(
                        Level::WARN,
                        "postgres consumer lagged {} ais messages",
                        num_lagged
                    );
                    AisProcessingAction::Continue
                }
            },
        }
    }
}

#[async_trait]
impl AisMigratorDestination for PostgresAdapter {
    async fn migrate_ais_data(
        &self,
        mmsi: i32,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> Result<(), InsertError> {
        self.add_ais_migration_data(mmsi, positions, progress)
            .await
            .change_context(InsertError)
    }
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> Result<Vec<AisVesselMigrate>, QueryError> {
        self.ais_vessel_migration_progress(migration_end_threshold)
            .await
            .change_context(QueryError)
    }
}

#[async_trait]
impl WebApiPort for PostgresAdapter {
    async fn ais_positions(
        &self,
        mmsi: i32,
        range: &DateRange,
    ) -> Result<Vec<AisPosition>, QueryError> {
        let positions = self
            .ais_positions_impl(mmsi, range)
            .await
            .change_context(QueryError)?;

        convert_models(positions).change_context(QueryError)
    }
}

#[async_trait]
impl ScraperInboundPort for PostgresAdapter {
    async fn add_landings(&self, _landings: Vec<NewLanding>) -> Result<(), InsertError> {
        unimplemented!();
    }
    async fn add_dca(&self, _dca: Vec<NewDca>) -> Result<(), InsertError> {
        unimplemented!();
    }
    async fn add_departure(&self, _departures: Vec<NewDeparture>) -> Result<(), InsertError> {
        unimplemented!();
    }
    async fn add_arrival(&self, _arrivals: Vec<NewArrival>) -> Result<(), InsertError> {
        unimplemented!();
    }
}

#[async_trait]
impl ScraperFileHashInboundPort for PostgresAdapter {
    async fn add(&self, _id: &FileHashId, _hash: String) -> Result<(), InsertError> {
        unimplemented!();
    }
    async fn diff(&self, _id: &FileHashId, _hash: &str) -> Result<HashDiff, QueryError> {
        unimplemented!();
    }
}

#[async_trait]
impl TripAssemblerOutboundPort for PostgresAdapter {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError> {
        unimplemented!();
    }
    async fn trip_calculation_timers(&self) -> Result<Vec<TripCalculationTimer>, QueryError> {
        unimplemented!();
    }
    async fn conflicts(
        &self,
        _id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, QueryError> {
        unimplemented!();
    }
    async fn landing_dates(
        &self,
        _vessel_id: i64,
        _start: &DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, QueryError> {
        unimplemented!();
    }
    async fn most_recent_trip(
        &self,
        _vessel_id: i64,
        _assembler_id: TripAssemblerId,
    ) -> Result<Option<Trip>, QueryError> {
        unimplemented!();
    }
    async fn departure_of_trip(&self, _trip_id: i64) -> Result<Departure, QueryError> {
        unimplemented!();
    }
    async fn ers_arrivals(
        &self,
        _vessel_id: i64,
        _start: &DateTime<Utc>,
        _filter: ArrivalFilter,
    ) -> Result<Arrival, QueryError> {
        unimplemented!();
    }
    async fn ers_departures(
        &self,
        _vessel_id: i64,
        _start: &DateTime<Utc>,
    ) -> Result<Departure, QueryError> {
        unimplemented!();
    }
    async fn trip_prior_to(
        &self,
        _vessel_id: i64,
        _assembler_id: TripAssemblerId,
        _time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, QueryError> {
        unimplemented!();
    }
}

#[async_trait]
impl TripAssemblerInboundPort for PostgresAdapter {
    async fn add_trips(
        &self,
        _vessel_id: i64,
        _new_trip_calculation_time: DateTime<Utc>,
        _conflict_strategy: TripsConflictStrategy,
        _trips: Vec<NewTrip>,
    ) -> Result<Vec<DateTime<Utc>>, InsertError> {
        unimplemented!();
    }
}

#[async_trait]
impl TripPrecisionOutboundPort for PostgresAdapter {
    async fn ports_of_trip(&self, _trip_id: i64) -> Result<TripPorts, QueryError> {
        unimplemented!();
    }
    async fn dock_points_of_trip(&self, _trip_id: i64) -> Result<TripDockPoints, QueryError> {
        unimplemented!();
    }
    async fn ais_positions(
        &self,
        _vessel_id: i64,
        _range: &DateRange,
    ) -> Result<Vec<AisPosition>, QueryError> {
        unimplemented!();
    }
    async fn trip_prior_to(
        &self,
        _vessel_id: i64,
        _assembler_id: TripAssemblerId,
        _time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, QueryError> {
        unimplemented!();
    }
    async fn delivery_points_associated_with_trip(
        &self,
        _trip_id: i64,
    ) -> Result<Vec<DeliveryPoint>, QueryError> {
        unimplemented!();
    }

    async fn trips_without_precision(
        &self,
        _vessel_id: i64,
        _assembler_id: TripAssemblerId,
    ) -> Result<Vec<Trip>, QueryError> {
        unimplemented!();
    }
}

#[async_trait]
impl TripPrecisionInboundPort for PostgresAdapter {
    async fn update_trip_precisions(
        &self,
        _updates: Vec<TripPrecisionUpdate>,
    ) -> Result<(), UpdateError> {
        unimplemented!();
    }
}

pub(crate) fn convert_models<D, I, C>(input: D) -> Result<Vec<C>, PostgresError>
where
    D: IntoIterator<Item = I>,
    C: TryFrom<I>,
    C: std::convert::TryFrom<I, Error = Report<PostgresError>>,
{
    input
        .into_iter()
        .map(C::try_from)
        .collect::<std::result::Result<Vec<_>, <C as std::convert::TryFrom<I>>::Error>>()
        .change_context(PostgresError::DataConversion)
}
