use crate::{
    error::PostgresError, ers_dca_set::ErsDcaSet, ers_dep_set::ErsDepSet, ers_por_set::ErsPorSet,
    ers_tra_set::ErsTraSet, landing_set::LandingSet,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, Result, ResultExt};
use futures::{Stream, StreamExt};
use kyogre_core::*;
use orca_core::{PsqlLogStatements, PsqlSettings};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use std::{collections::HashMap, pin::Pin};
use tracing::{event, instrument, Level};

#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
    pub(crate) ais_pool: PgPool,
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
            .port(settings.port as u16);

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

        let ais_opts = opts
            .clone()
            .options([("plan_cache_mode", "force_custom_plan")]);

        let pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(20))
            .connect_with(opts)
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        let ais_pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(20))
            .connect_with(ais_opts)
            .await
            .into_report()
            .change_context(PostgresError::Connection)?;

        Ok(PostgresAdapter { pool, ais_pool })
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

    pub(crate) async fn begin(
        &self,
    ) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, PostgresError> {
        self.pool
            .begin()
            .await
            .into_report()
            .change_context(PostgresError::Transaction)
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
    fn ais_positions(
        &self,
        mmsi: i32,
        range: &DateRange,
    ) -> PinBoxStream<'_, AisPosition, QueryError> {
        convert_stream(self.ais_positions_impl(mmsi, range)).boxed()
    }

    fn species_groups(&self) -> PinBoxStream<'_, SpeciesGroup, QueryError> {
        convert_stream(self.species_groups_impl()).boxed()
    }

    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir, QueryError> {
        convert_stream(self.species_fiskeridir_impl()).boxed()
    }
    fn species(&self) -> PinBoxStream<'_, Species, QueryError> {
        convert_stream(self.species_impl()).boxed()
    }
    fn species_main_groups(&self) -> PinBoxStream<'_, SpeciesMainGroup, QueryError> {
        convert_stream(self.species_main_groups_impl()).boxed()
    }
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao, QueryError> {
        convert_stream(self.species_fao_impl()).boxed()
    }

    fn vessels(&self) -> Pin<Box<dyn Stream<Item = Result<Vessel, QueryError>> + Send + '_>> {
        convert_stream(self.fiskeridir_ais_vessel_combinations()).boxed()
    }

    fn hauls(&self, query: HaulsQuery) -> Result<PinBoxStream<'_, Haul, QueryError>, QueryError> {
        let stream = self.hauls_impl(query).change_context(QueryError)?;
        Ok(convert_stream(stream).boxed())
    }

    async fn hauls_grid(&self, query: HaulsQuery) -> Result<HaulsGrid, QueryError> {
        let grid = self
            .hauls_grid_impl(query)
            .await
            .change_context(QueryError)?;

        HaulsGrid::try_from(grid).change_context(QueryError)
    }

    async fn trip_of_haul(&self, haul_id: &str) -> Result<Option<Trip>, QueryError> {
        convert_optional(
            self.trip_of_haul_impl(haul_id)
                .await
                .change_context(QueryError)?,
        )
    }
}

#[async_trait]
impl ScraperInboundPort for PostgresAdapter {
    async fn add_landings(&self, landings: Vec<fiskeridir_rs::Landing>) -> Result<(), InsertError> {
        let set = LandingSet::new(landings).change_context(InsertError)?;

        self.add_landing_set(set).await.change_context(InsertError)
    }
    async fn delete_ers_dca(&self, year: u32) -> Result<(), DeleteError> {
        self.delete_ers_dca_impl(year)
            .await
            .change_context(DeleteError)
    }
    async fn add_ers_dca(&self, ers_dca: Vec<fiskeridir_rs::ErsDca>) -> Result<(), InsertError> {
        let set = ErsDcaSet::new(ers_dca).change_context(InsertError)?;
        self.add_ers_dca_set(set).await.change_context(InsertError)
    }
    async fn add_ers_dep(&self, ers_dep: Vec<fiskeridir_rs::ErsDep>) -> Result<(), InsertError> {
        let set = ErsDepSet::new(ers_dep).change_context(InsertError)?;
        self.add_ers_dep_set(set).await.change_context(InsertError)
    }
    async fn delete_ers_dep_catches(&self, year: u32) -> Result<(), DeleteError> {
        self.delete_ers_dep_catches_impl(year)
            .await
            .change_context(DeleteError)
    }
    async fn add_ers_por(&self, ers_por: Vec<fiskeridir_rs::ErsPor>) -> Result<(), InsertError> {
        let set = ErsPorSet::new(ers_por).change_context(InsertError)?;
        self.add_ers_por_set(set).await.change_context(InsertError)
    }
    async fn delete_ers_por_catches(&self, year: u32) -> Result<(), DeleteError> {
        self.delete_ers_por_catches_impl(year)
            .await
            .change_context(DeleteError)
    }
    async fn add_ers_tra(&self, ers_tra: Vec<fiskeridir_rs::ErsTra>) -> Result<(), InsertError> {
        let set = ErsTraSet::new(ers_tra).change_context(InsertError)?;
        self.add_ers_tra_set(set).await.change_context(InsertError)
    }
    async fn delete_ers_tra_catches(&self, year: u32) -> Result<(), DeleteError> {
        self.delete_ers_tra_catches_impl(year)
            .await
            .change_context(DeleteError)
    }
    async fn update_database_views(&self) -> Result<(), UpdateError> {
        self.update_database_views_impl()
            .await
            .change_context(UpdateError)
    }
}

#[async_trait]
impl ScraperFileHashInboundPort for PostgresAdapter {
    async fn add(&self, id: &FileHashId, hash: String) -> Result<(), InsertError> {
        self.add_hash(id, hash).await.change_context(InsertError)
    }
    async fn diff(&self, id: &FileHashId, hash: &str) -> Result<HashDiff, QueryError> {
        self.diff_hash(id, hash).await.change_context(QueryError)
    }
}

#[async_trait]
impl TripAssemblerOutboundPort for PostgresAdapter {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError> {
        let mut stream = convert_stream(self.fiskeridir_ais_vessel_combinations());

        let mut vessels = vec![];

        while let Some(v) = stream.next().await {
            vessels.push(v.change_context_lazy(|| QueryError)?);
        }

        Ok(vessels)
    }
    async fn trip_calculation_timers(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripCalculationTimer>, QueryError> {
        self.trip_calculation_timers_impl(trip_assembler_id)
            .await
            .change_context(QueryError)
    }
    async fn conflicts(
        &self,
        id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, QueryError> {
        self.trip_assembler_conflicts(id)
            .await
            .change_context(QueryError)
    }
    async fn landing_dates(
        &self,
        vessel_id: i64,
        start: &DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, QueryError> {
        self.landing_dates_impl(vessel_id, start)
            .await
            .change_context(QueryError)
    }
    async fn most_recent_trip(
        &self,
        fiskeridir_vessel_id: i64,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Option<Trip>, QueryError> {
        convert_optional(
            self.most_recent_trip_impl(fiskeridir_vessel_id, trip_assembler_id)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn ers_arrivals(
        &self,
        fiskeridir_vessel_id: i64,
        start: &DateTime<Utc>,
        filter: ArrivalFilter,
    ) -> Result<Vec<Arrival>, QueryError> {
        self.ers_arrivals_impl(fiskeridir_vessel_id, start, filter)
            .await
            .change_context(QueryError)
    }
    async fn ers_departures(
        &self,
        fiskeridir_vessel_id: i64,
        start: &DateTime<Utc>,
    ) -> Result<Vec<Departure>, QueryError> {
        self.ers_departures_impl(fiskeridir_vessel_id, start)
            .await
            .change_context(QueryError)
    }
    async fn trip_at_or_prior_to(
        &self,
        fiskeridir_vessel_id: i64,
        trip_assembler_id: TripAssemblerId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, QueryError> {
        convert_optional(
            self.trip_at_or_prior_to_impl(fiskeridir_vessel_id, trip_assembler_id, time)
                .await
                .change_context(QueryError)?,
        )
    }
}

#[async_trait]
impl TripAssemblerInboundPort for PostgresAdapter {
    async fn add_trips(
        &self,
        fiskeridir_vessel_id: i64,
        new_trip_calculation_time: DateTime<Utc>,
        conflict_strategy: TripsConflictStrategy,
        trips: Vec<NewTrip>,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<(), InsertError> {
        self.add_trips_impl(
            fiskeridir_vessel_id,
            new_trip_calculation_time,
            conflict_strategy,
            trips,
            trip_assembler_id,
        )
        .await
        .change_context(InsertError)
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

pub(crate) fn convert_stream<I, A, B>(input: I) -> impl Stream<Item = Result<B, QueryError>>
where
    I: Stream<Item = Result<A, PostgresError>>,
    B: TryFrom<A>,
    B: std::convert::TryFrom<A, Error = Report<PostgresError>>,
{
    input.map(|i| {
        match i {
            Ok(i) => B::try_from(i),
            Err(e) => Err(e),
        }
        .change_context(QueryError)
    })
}

pub(crate) fn convert_optional<A, B>(val: Option<A>) -> Result<Option<B>, QueryError>
where
    B: std::convert::TryFrom<A, Error = Report<PostgresError>>,
{
    val.map(|a| B::try_from(a))
        .transpose()
        .change_context(QueryError)
}
