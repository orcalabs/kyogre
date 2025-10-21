use crate::{
    error::{Error, Result},
    queries::ais::AisPositionsArg,
};
use async_channel::Receiver;
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use fiskeridir_rs::{CallSign, DataFileId, LandingId, OrgId, SpeciesGroup};
use futures::{Stream, StreamExt, TryStreamExt};
use kyogre_core::*;
use orca_core::{Environment, PsqlLogStatements, PsqlSettings};
use sqlx::{
    ConnectOptions, PgConnection, PgPool,
    migrate::Migrator,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use std::{fmt::Debug, path::PathBuf, result::Result as StdResult};
use tokio::time::timeout;
use tracing::{error, info, instrument, warn};

#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
    no_plan_cache_pool: Option<PgPool>,
    pub(crate) ignored_conflict_call_signs: Vec<String>,
    pub(crate) environment: Environment,
}

pub enum ConsumeLoopOutcome {
    Exit,
    Continue,
}

enum AisProcessingAction {
    Exit,
    Continue,
    Retry {
        positions: Option<Vec<NewAisPosition>>,
        static_messages: Option<Vec<NewAisStatic>>,
    },
}

impl Debug for AisProcessingAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exit => write!(f, "Exit"),
            Self::Continue => write!(f, "Continue"),
            Self::Retry {
                positions,
                static_messages,
            } => f
                .debug_struct("Retry")
                .field("num_positions", &positions.as_ref().map(|v| v.len()))
                .field(
                    "num_static_messages",
                    &static_messages.as_ref().map(|v| v.len()),
                )
                .finish(),
        }
    }
}

impl PostgresAdapter {
    pub async fn new(settings: &PsqlSettings) -> Result<PostgresAdapter> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .ok()
            .and_then(|v| v.try_into().ok())
            .unwrap_or(Environment::Test);

        let mut connections_per_pool = (settings.max_connections / 2) as u32;
        if connections_per_pool == 0 {
            connections_per_pool = 1;
        }

        let mut opts = PgConnectOptions::new()
            .username(&settings.username)
            .host(&settings.ip)
            .port(settings.port as u16);

        if let Some(password) = &settings.password {
            opts = opts.password(password);
        }

        if let Some(db_name) = &settings.db_name {
            opts = opts.database(db_name);
        }

        if let Some(app_name) = &settings.application_name {
            opts = opts.application_name(app_name);
        }

        if let Some(root_cert_path) = &settings.root_cert {
            opts = opts
                .ssl_root_cert(root_cert_path)
                .ssl_mode(PgSslMode::VerifyFull);
        }

        match settings.log_statements {
            PsqlLogStatements::Enable => (),
            PsqlLogStatements::Disable => {
                opts = opts.disable_statement_logging();
            }
        }

        if environment == Environment::Test {
            opts = opts.ssl_mode(PgSslMode::Disable);
        }

        let no_plan_cache_pool = match environment {
            Environment::Development | Environment::Production | Environment::Local => {
                let opts = opts
                    .clone()
                    .options([("plan_cache_mode", "force_custom_plan")]);

                let pool = PgPoolOptions::new()
                    .min_connections(1)
                    .max_connections(connections_per_pool)
                    .acquire_timeout(std::time::Duration::from_secs(60))
                    .after_connect(|conn, _meta| Box::pin(check_if_read_only_connection(conn)))
                    .connect_with(opts)
                    .await?;

                Some(pool)
            }
            Environment::OnPremise | Environment::Test => None,
        };

        let pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .after_connect(|conn, _meta| Box::pin(check_if_read_only_connection(conn)))
            .connect_with(opts)
            .await?;

        let ignored_conflict_call_signs: Vec<String> = IGNORED_CONFLICT_CALL_SIGNS
            .iter()
            .map(|v| v.to_string())
            .collect();

        Ok(PostgresAdapter {
            pool,
            no_plan_cache_pool,
            ignored_conflict_call_signs,
            environment,
        })
    }

    pub async fn close(self) {
        self.pool.close().await
    }

    pub(crate) fn no_plan_cache_pool(&self) -> &PgPool {
        match self.environment {
            Environment::Production
            | Environment::Development
            | Environment::OnPremise
            | Environment::Local => self.no_plan_cache_pool.as_ref().unwrap(),
            Environment::Test => &self.pool,
        }
    }

    pub async fn reset_models(&self, models: &[ModelId]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let models: Vec<i32> = models.iter().map(|v| *v as i32).collect();

        sqlx::query!(
            r#"
DELETE FROM ml_hauls_training_log
WHERE
    ml_model_id = ANY ($1)
            "#,
            &models
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM fishing_spot_predictions
WHERE
    ml_model_id = ANY ($1)
            "#,
            &models
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
DELETE FROM fishing_weight_predictions
WHERE
    ml_model_id = ANY ($1)
            "#,
            &models
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn do_migrations(&self) {
        sqlx::migrate!()
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .unwrap();
        info!("ran db migrations successfully");
    }

    pub async fn do_migrations_path(&self, path: PathBuf) {
        Migrator::new(path)
            .await
            .unwrap()
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .unwrap();
        info!("ran db migrations successfully");
    }

    #[instrument(skip_all)]
    pub async fn consume_loop_iteration(
        &self,
        receiver: &mut Receiver<DataMessage>,
        process_confirmation: Option<&tokio::sync::mpsc::Sender<()>>,
    ) -> ConsumeLoopOutcome {
        let time = std::time::Duration::from_secs(60 * 20);
        let message = receiver.recv().await;
        let result = timeout(time, self.process_message(message)).await;
        // Only enabled in tests
        if let Some(s) = process_confirmation {
            // Only error here is if the reciver is closed which will happen in failing tests.
            // Not unwrapping here to avoid confusion on what actually went wrong in a failed test.
            let _ = s.send(()).await;
        }

        let result = match result {
            Ok(r) => r,
            Err(e) => panic!("ais insertion took more than {e}"),
        };

        match result {
            AisProcessingAction::Exit => ConsumeLoopOutcome::Exit,
            AisProcessingAction::Continue => ConsumeLoopOutcome::Continue,
            AisProcessingAction::Retry {
                positions,
                static_messages,
            } => {
                let mut err = Ok(());
                for _ in 0..2 {
                    let timeout_err = timeout(
                        time,
                        self.insertion_retry(
                            &mut positions.as_deref(),
                            &mut static_messages.as_deref(),
                        ),
                    )
                    .await;

                    err = match timeout_err {
                        Ok(r) => r,
                        Err(e) => panic!("ais retry insertion took more than {e}"),
                    };

                    if err.is_ok() {
                        break;
                    }
                }
                if let Err(e) = err {
                    error!(
                        "failed insertion retry for ais data, contained positions: {},
                            contained static_messages: {}, err: {e:?}",
                        positions.is_some(),
                        static_messages.is_some()
                    );
                }
                ConsumeLoopOutcome::Continue
            }
        }
    }

    pub async fn consume_loop(
        &self,
        mut receiver: Receiver<DataMessage>,
        process_confirmation: Option<tokio::sync::mpsc::Sender<()>>,
    ) {
        loop {
            match self
                .consume_loop_iteration(&mut receiver, process_confirmation.as_ref())
                .await
            {
                ConsumeLoopOutcome::Exit => break,
                ConsumeLoopOutcome::Continue => continue,
            }
        }
    }

    #[instrument(skip_all, name = "postgres_insertion_retry")]
    async fn insertion_retry(
        &self,
        positions: &mut Option<&[NewAisPosition]>,
        static_messages: &mut Option<&[NewAisStatic]>,
    ) -> Result<()> {
        let res = if let Some(pos) = positions {
            let res = self.add_ais_positions(pos).await;
            if res.is_ok() {
                *positions = None;
            }
            res
        } else {
            Ok(())
        };

        let res2 = if let Some(new_statics) = static_messages {
            let res = self.add_ais_vessels(new_statics).await;
            if res.is_ok() {
                *static_messages = None;
            }
            res
        } else {
            Ok(())
        };

        res.and(res2)
    }

    #[instrument(skip_all, ret, name = "postgres_insert_ais_data")]
    async fn process_message(
        &self,
        incoming: StdResult<DataMessage, async_channel::RecvError>,
    ) -> AisProcessingAction {
        match incoming {
            Ok(message) => {
                match (
                    self.add_ais_positions(&message.positions).await,
                    self.add_ais_vessels(&message.static_messages).await,
                ) {
                    (Ok(_), Ok(_)) => AisProcessingAction::Continue,
                    (Ok(_), Err(e)) => {
                        warn!("failed to add ais vessels, err: {e:?}");
                        AisProcessingAction::Retry {
                            positions: None,
                            static_messages: Some(message.static_messages),
                        }
                    }
                    (Err(e), Ok(_)) => {
                        warn!("failed to add ais positions, err: {e:?}");
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            static_messages: None,
                        }
                    }
                    (Err(e1), Err(e2)) => {
                        warn!("failed to add ais positions, err: {e1:?}");
                        warn!("failed to add ais vessels, err: {e2:?}");
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            static_messages: Some(message.static_messages),
                        }
                    }
                }
            }
            Err(e) => {
                warn!("sender half of ais broadcast channel closed unexpectedly: '{e:?}', exiting");
                AisProcessingAction::Exit
            }
        }
    }
}

#[cfg(feature = "test")]
impl TestStorage for PostgresAdapter {}

#[cfg(feature = "test")]
#[async_trait]
impl TestHelperOutbound for PostgresAdapter {
    async fn all_fuel_measurement_ranges(&self) -> Vec<FuelMeasurementRange> {
        self.all_fuel_measurement_ranges_impl().await.unwrap()
    }
    async fn unprocessed_trips(&self) -> u32 {
        self.unprocessed_trips_impl().await.unwrap()
    }
    async fn fuel_estimates_with_status(&self, status: ProcessingStatus) -> u32 {
        self.fuel_estimates_with_status_impl(status).await.unwrap()
    }

    async fn all_fuel_estimates(&self) -> Vec<f64> {
        self.all_fuel_estimates_impl().await.unwrap()
    }

    async fn sum_fuel_estimates(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        to_skip: &[NaiveDate],
        vessels: Option<&[FiskeridirVesselId]>,
    ) -> f64 {
        self.sum_fuel_estimates_impl(start, end, to_skip, vessels)
            .await
            .unwrap()
    }

    async fn trips_with_benchmark_status(&self, status: ProcessingStatus) -> u32 {
        self.trips_with_benchmark_status_impl(status).await.unwrap()
    }
    async fn trip_assembler_log(&self) -> Vec<TripAssemblerLogEntry> {
        self.trip_assembler_log_impl()
            .map(|v| v.unwrap().try_into().unwrap())
            .collect()
            .await
    }
    async fn active_vessel_conflicts(&self) -> Vec<ActiveVesselConflict> {
        self.active_vessel_conflicts_impl()
            .try_collect()
            .await
            .unwrap()
    }
    async fn all_tra(&self) -> Vec<Tra> {
        self.all_ers_tra_impl()
            .await
            .unwrap()
            .into_iter()
            .map(|t| Tra::try_from(t).unwrap())
            .collect()
    }
    async fn all_dep(&self) -> Vec<Departure> {
        self.all_ers_departures_impl().await.unwrap()
    }
    async fn all_por(&self) -> Vec<Arrival> {
        self.all_ers_arrivals_impl().await.unwrap()
    }

    async fn delivery_points_log(&self) -> Vec<serde_json::Value> {
        self.delivery_points_log_impl().await.unwrap()
    }

    async fn all_ais(&self) -> Vec<AisPosition> {
        self.all_ais_positions_impl(AisPositionsArg::All)
            .await
            .unwrap()
    }
    async fn all_vms(&self) -> Vec<VmsPosition> {
        self.all_vms_impl()
            .map(|v| v.unwrap().into())
            .collect()
            .await
    }
    async fn all_ais_vms(&self) -> Vec<AisVmsPosition> {
        self.all_ais_vms_impl().await.unwrap()
    }
    async fn port(&self, port_id: &str) -> Option<Port> {
        self.port_impl(port_id)
            .await
            .unwrap()
            .map(Port::try_from)
            .transpose()
            .unwrap()
    }
    async fn delivery_point(&self, id: &fiskeridir_rs::DeliveryPointId) -> Option<DeliveryPoint> {
        self.delivery_point_impl(id).await.unwrap()
    }
    async fn dock_points_of_port(&self, port_id: &str) -> Vec<PortDockPoint> {
        self.dock_points_of_port_impl(port_id).await.unwrap()
    }
}

#[async_trait]
impl DailyWeatherInbound for PostgresAdapter {
    async fn dirty_dates(&self) -> CoreResult<Vec<NaiveDate>> {
        Ok(self.dirty_dates_impl().await?)
    }
    async fn prune_dirty_dates(&self) -> CoreResult<()> {
        Ok(self.prune_dirty_dates_impl().await?)
    }
    async fn catch_locations_with_weather(&self) -> CoreResult<Vec<CatchLocationId>> {
        Ok(self.catch_locations_with_weather_impl().await?)
    }

    async fn update_daily_weather(
        &self,
        catch_locations: &[CatchLocationId],
        date: NaiveDate,
    ) -> CoreResult<()> {
        Ok(self
            .update_daily_weather_impl(catch_locations, date)
            .await?)
    }
}

#[cfg(feature = "test")]
#[async_trait]
impl TestHelperInbound for PostgresAdapter {
    async fn manual_vessel_conflict_override(&self, overrides: Vec<NewVesselConflict>) {
        self.manual_conflict_override_impl(overrides).await.unwrap();
    }
    async fn queue_trip_reset(&self) {
        self.queue_trip_reset_impl().await.unwrap();
    }
    async fn clear_trip_distancing(&self, vessel_id: FiskeridirVesselId) {
        self.clear_trip_distancing_impl(vessel_id).await.unwrap();
    }
    async fn clear_trip_precision(&self, vessel_id: FiskeridirVesselId) {
        self.clear_trip_precision_impl(vessel_id).await.unwrap();
    }
    async fn add_manual_delivery_points(&self, values: Vec<ManualDeliveryPoint>) {
        self.add_manual_delivery_points_impl(values).await.unwrap();
    }
    async fn add_deprecated_delivery_point(
        &self,
        old: fiskeridir_rs::DeliveryPointId,
        new: fiskeridir_rs::DeliveryPointId,
    ) -> CoreResult<()> {
        Ok(self.add_deprecated_delivery_point_impl(old, new).await?)
    }
}

#[async_trait]
impl AisConsumeLoop for PostgresAdapter {
    async fn consume(
        &self,
        receiver: Receiver<DataMessage>,
        process_confirmation: Option<tokio::sync::mpsc::Sender<()>>,
    ) {
        self.consume_loop(receiver, process_confirmation).await
    }
}

#[async_trait]
impl AisMigratorSource for PostgresAdapter {
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CoreResult<Vec<AisPosition>> {
        Ok(self
            .all_ais_positions_impl(AisPositionsArg::Filter { mmsi, start, end })
            .await?)
    }
    async fn existing_mmsis(&self) -> CoreResult<Vec<Mmsi>> {
        Ok(self.existing_mmsis_impl().await?)
    }
}

#[async_trait]
impl AisMigratorDestination for PostgresAdapter {
    async fn add_mmsis(&self, mmsis: &[Mmsi]) -> CoreResult<()> {
        self.add_mmsis_impl(mmsis).await?;
        Ok(())
    }
    async fn migrate_ais_data(
        &self,
        mmsi: Mmsi,
        positions: Vec<AisPosition>,
        progress: DateTime<Utc>,
    ) -> CoreResult<()> {
        self.add_ais_migration_data(mmsi, positions, progress)
            .await?;
        Ok(())
    }
    async fn vessel_migration_progress(
        &self,
        migration_end_threshold: &DateTime<Utc>,
    ) -> CoreResult<Vec<AisVesselMigrate>> {
        Ok(self
            .ais_vessel_migration_progress(migration_end_threshold)
            .await?)
    }
}

#[async_trait]
impl FuelEstimation for PostgresAdapter {
    #[cfg(feature = "test")]
    async fn latest_position(&self) -> CoreResult<Option<NaiveDate>> {
        Ok(self.latest_position_impl().await?)
    }

    async fn last_run(&self) -> CoreResult<Option<DateTime<Utc>>> {
        Ok(retry(|| self.last_run_impl(Processor::FuelProcessor)).await?)
    }

    async fn add_run(&self) -> CoreResult<()> {
        Ok(retry(|| self.add_run_impl(Processor::FuelProcessor)).await?)
    }

    async fn add_fuel_estimates(&self, estimates: &[NewFuelDayEstimate]) -> CoreResult<()> {
        Ok(retry(|| self.add_fuel_estimates_impl(estimates)).await?)
    }
    async fn vessels_with_trips(&self, num_trips: u32) -> CoreResult<Vec<Vessel>> {
        Ok(retry(|| async {
            let out: CoreResult<Vec<Vessel>> = self
                .vessels_with_trips_impl(num_trips)
                .try_convert_collect()
                .await;
            out
        })
        .await?)
    }
    async fn delete_fuel_estimates(&self, vessels: &[FiskeridirVesselId]) -> CoreResult<()> {
        Ok(retry(|| self.delete_fuel_estimates_impl(vessels)).await?)
    }
    async fn reset_trip_positions_fuel_status(
        &self,
        vessels: &[FiskeridirVesselId],
    ) -> CoreResult<()> {
        Ok(retry(|| self.reset_trip_positions_fuel_status_impl(vessels)).await?)
    }
    async fn dates_to_estimate(
        &self,
        vessel_id: FiskeridirVesselId,
        call_sign: Option<&CallSign>,
        mmsi: Option<Mmsi>,
        end_date: NaiveDate,
    ) -> CoreResult<Vec<NaiveDate>> {
        Ok(retry(|| self.dates_to_estimate_impl(vessel_id, call_sign, mmsi, end_date)).await?)
    }

    async fn fuel_estimation_positions(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<DailyFuelEstimationPosition>> {
        Ok(
            retry(|| self.fuel_estimation_positions_impl(vessel_id, mmsi, call_sign, range))
                .await?,
        )
    }

    async fn vessel_max_cargo_weight(&self, vessel_id: FiskeridirVesselId) -> CoreResult<f64> {
        Ok(retry(|| self.vessel_max_cargo_weight_impl(vessel_id)).await?)
    }
}

#[async_trait]
impl CurrentPositionInbound for PostgresAdapter {
    async fn update_current_positions(
        &self,
        updates: Vec<CurrentPositionsUpdate>,
    ) -> CoreResult<()> {
        Ok(retry(|| self.update_current_positions_impl(&updates)).await?)
    }
}

#[async_trait]
impl CurrentPositionOutbound for PostgresAdapter {
    async fn vessels(&self) -> CoreResult<Vec<CurrentPositionVessel>> {
        Ok(retry(|| self.current_position_vessels_impl()).await?)
    }
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        Ok(retry(|| {
            self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All)
                .try_collect()
        })
        .await?)
    }
}

#[async_trait]
impl LiveFuelInbound for PostgresAdapter {
    async fn delete_old_live_fuel(
        &self,
        fiskeridir_vessel_id: FiskeridirVesselId,
        threshold: DateTime<Utc>,
    ) -> CoreResult<()> {
        Ok(retry(|| self.delete_old_live_fuel_impl(fiskeridir_vessel_id, threshold)).await?)
    }
    async fn add_live_fuel(
        &self,
        vessel_id: FiskeridirVesselId,
        fuel: &[NewLiveFuel],
    ) -> CoreResult<()> {
        Ok(retry(|| self.add_live_fuel_impl(vessel_id, fuel)).await?)
    }
    async fn live_fuel_vessels(&self) -> CoreResult<Vec<LiveFuelVessel>> {
        Ok(retry(|| self.live_fuel_vessels_impl()).await?)
    }
    async fn ais_positions(&self, mmsi: Mmsi, range: &DateRange) -> CoreResult<Vec<AisPosition>> {
        Ok(self
            .all_ais_positions_impl(AisPositionsArg::Filter {
                mmsi,
                start: range.start(),
                end: range.end(),
            })
            .await?)
    }
}

#[async_trait]
impl WebApiOutboundPort for PostgresAdapter {
    async fn fui(&self, query: FuiQuery) -> WebApiResult<Option<f64>> {
        Ok(retry(|| self.fui_impl(&query)).await?)
    }
    async fn average_fui(&self, query: AverageFuiQuery) -> WebApiResult<Option<f64>> {
        Ok(retry(|| self.average_fui_impl(&query)).await?)
    }
    async fn live_fuel(&self, query: &LiveFuelQuery) -> WebApiResult<LiveFuel> {
        Ok(retry(|| async {
            let entries: WebApiResult<Vec<kyogre_core::LiveFuelEntry>> =
                self.live_fuel_impl(query).convert_collect().await;
            let entries = entries?;
            Ok::<kyogre_core::LiveFuel, kyogre_core::WebApiError>(kyogre_core::LiveFuel {
                total_fuel_liter: entries.iter().map(|e| e.fuel_liter).sum(),
                entries,
            })
        })
        .await?)
    }
    async fn org_benchmarks(
        &self,
        query: &OrgBenchmarkQuery,
    ) -> WebApiResult<Option<OrgBenchmarks>> {
        convert_optional(retry(|| self.org_benchmarks_impl(query)).await?)
    }
    async fn fuel_estimation(&self, query: &FuelQuery) -> WebApiResult<f64> {
        Ok(retry(|| self.fuel_estimation_impl(query)).await?)
    }
    async fn fuel_estimation_by_org(
        &self,
        query: &FuelQuery,
        org_id: OrgId,
    ) -> WebApiResult<Option<Vec<FuelEntry>>> {
        Ok(retry(|| self.fuel_estimation_by_org_impl(query, org_id)).await?)
    }
    async fn update_vessel(
        &self,
        call_sign: &CallSign,
        update: &UpdateVessel,
    ) -> WebApiResult<Option<Vessel>> {
        Ok(retry(|| self.update_vessel_impl(call_sign, update)).await?)
    }
    async fn average_trip_benchmarks(
        &self,
        query: AverageTripBenchmarksQuery,
    ) -> WebApiResult<AverageTripBenchmarks> {
        Ok(retry(|| self.average_trip_benchmarks_impl(&query)).await?)
    }
    async fn average_eeoi(&self, query: AverageEeoiQuery) -> WebApiResult<Option<f64>> {
        Ok(retry(|| self.average_eeoi_impl(&query)).await?)
    }
    fn current_positions(
        &self,
        limit: Option<DateTime<Utc>>,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, CurrentPosition> {
        self.current_positions_impl(limit, user_policy)
            .map_err(|e| e.into())
            .boxed()
    }
    fn current_trip_positions(
        &self,
        vessel_id: FiskeridirVesselId,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition> {
        self.current_trip_positions_impl(vessel_id, user_policy)
            .map_err(|e| e.into())
            .boxed()
    }
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        permission: AisPermission,
    ) -> PinBoxStream<'_, AisPosition> {
        self.ais_positions_impl(mmsi, range, permission)
            .map_err(|e| e.into())
            .boxed()
    }
    fn vms_positions<'a>(
        &'a self,
        call_sign: &'a CallSign,
        range: &'a DateRange,
    ) -> PinBoxStream<'a, VmsPosition> {
        self.vms_positions_impl(call_sign, range).convert().boxed()
    }

    fn ais_vms_positions(
        &self,
        params: AisVmsParams,
        permission: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition> {
        match params {
            AisVmsParams::Trip(trip_id) => self
                .trip_positions_with_inside_haul_impl(trip_id, permission)
                .map_err(|e| e.into())
                .boxed(),
            AisVmsParams::Range {
                mmsi,
                call_sign,
                range,
            } => self
                .ais_vms_positions_impl(mmsi, call_sign.as_ref(), &range, permission)
                .map_err(|e| e.into())
                .boxed(),
        }
    }
    fn species(&self) -> PinBoxStream<'_, Species> {
        self.species_impl().convert().boxed()
    }
    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir> {
        self.species_fiskeridir_impl().convert().boxed()
    }
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao> {
        self.species_fao_impl().map_err(|e| e.into()).boxed()
    }

    fn vessels(&self) -> PinBoxStream<'_, Vessel> {
        self.fiskeridir_ais_vessel_combinations()
            .try_convert()
            .boxed()
    }

    fn hauls(&self, query: HaulsQuery) -> PinBoxStream<'_, Haul> {
        self.hauls_impl(query).try_convert().boxed()
    }

    async fn vessel_benchmarks(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
    ) -> WebApiResult<VesselBenchmarks> {
        Ok(retry(|| self.vessel_benchmarks_impl(user_id, call_sign))
            .await?
            .try_into()?)
    }
    async fn trip_benchmarks(
        &self,
        query: TripBenchmarksQuery,
    ) -> WebApiResult<Vec<TripWithBenchmark>> {
        Ok(retry(|| self.trip_benchmarks_impl(&query)).await?)
    }
    async fn eeoi(&self, query: EeoiQuery) -> WebApiResult<Option<f64>> {
        Ok(retry(|| self.eeoi_impl(&query)).await?)
    }
    fn detailed_trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> PinBoxStream<'_, TripDetailed> {
        self.detailed_trips_impl(query, read_fishing_facility)
            .try_convert()
            .boxed()
    }

    async fn current_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> WebApiResult<Option<CurrentTrip>> {
        convert_optional(retry(|| self.current_trip_impl(vessel_id, read_fishing_facility)).await?)
    }

    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> WebApiResult<HaulsMatrix> {
        Ok(retry(|| self.hauls_matrix_impl(query)).await?)
    }

    fn landings(&self, query: LandingsQuery) -> PinBoxStream<'_, Landing> {
        self.landings_impl(query).try_convert().boxed()
    }

    async fn landing_matrix(&self, query: &LandingMatrixQuery) -> WebApiResult<LandingMatrix> {
        Ok(retry(|| self.landing_matrix_impl(query)).await?)
    }

    fn fishing_facilities(
        &self,
        query: FishingFacilitiesQuery,
    ) -> PinBoxStream<'_, FishingFacility> {
        self.fishing_facilities_impl(query)
            .map_err(|e| e.into())
            .boxed()
    }

    async fn get_user(&self, user_id: BarentswatchUserId) -> WebApiResult<Option<User>> {
        Ok(retry(|| self.get_user_impl(user_id)).await?)
    }

    fn delivery_points(&self) -> PinBoxStream<'_, DeliveryPoint> {
        self.delivery_points_impl().map_err(|e| e.into()).boxed()
    }

    fn weather(&self, query: WeatherQuery) -> PinBoxStream<'_, Weather> {
        self.weather_impl(query).map_err(|e| e.into()).boxed()
    }

    fn weather_locations(&self) -> PinBoxStream<'_, WeatherLocation> {
        self.weather_locations_impl().try_convert().boxed()
    }

    async fn fishing_spot_prediction(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
    ) -> WebApiResult<Option<FishingSpotPrediction>> {
        Ok(retry(|| self.fishing_spot_prediction_impl(model_id, species, date)).await?)
    }

    fn fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
        limit: u32,
    ) -> PinBoxStream<'_, FishingWeightPrediction> {
        self.fishing_weight_predictions_impl(model_id, species, date, limit)
            .map_err(|e| e.into())
            .boxed()
    }

    fn all_fishing_spot_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingSpotPrediction> {
        self.all_fishing_spot_predictions_impl(model_id)
            .map_err(|e| e.into())
            .boxed()
    }

    fn all_fishing_weight_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingWeightPrediction> {
        self.all_fishing_weight_predictions_impl(model_id)
            .map_err(|e| e.into())
            .boxed()
    }

    fn fuel_measurements(&self, query: FuelMeasurementsQuery) -> PinBoxStream<'_, FuelMeasurement> {
        self.fuel_measurements_impl(query)
            .map_err(|e| e.into())
            .boxed()
    }
}

#[async_trait]
impl WebApiInboundPort for PostgresAdapter {
    async fn update_user(&self, user: &User) -> WebApiResult<()> {
        retry(|| self.update_user_impl(user)).await?;
        Ok(())
    }
    async fn add_fuel_measurements(
        &self,
        measurements: &[CreateFuelMeasurement],
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> WebApiResult<Vec<FuelMeasurement>> {
        Ok(retry(|| self.add_fuel_measurements_impl(measurements, call_sign, user_id)).await?)
    }
    async fn update_fuel_measurements(
        &self,
        measurements: &[FuelMeasurement],
        call_sign: &CallSign,
        user_id: BarentswatchUserId,
    ) -> WebApiResult<()> {
        retry(|| self.update_fuel_measurements_impl(measurements, call_sign, user_id)).await?;
        Ok(())
    }
    async fn delete_fuel_measurements(
        &self,
        measurements: &[DeleteFuelMeasurement],
        call_sign: &CallSign,
    ) -> WebApiResult<()> {
        retry(|| self.delete_fuel_measurements_impl(measurements, call_sign)).await?;
        Ok(())
    }
}

#[async_trait]
impl ScraperInboundPort for PostgresAdapter {
    async fn add_fishing_facilities(&self, facilities: Vec<FishingFacility>) -> CoreResult<()> {
        self.add_fishing_facilities_impl(facilities).await?;
        Ok(())
    }
    async fn add_weekly_sales(&self, weekly_sales: Vec<WeeklySale>) -> CoreResult<()> {
        self.add_weekly_sales_impl(weekly_sales).await?;
        Ok(())
    }
    async fn add_register_vessels(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> CoreResult<()> {
        self.add_register_vessels_full(vessels).await?;
        Ok(())
    }
    async fn add_buyer_locations(&self, locations: Vec<BuyerLocation>) -> CoreResult<()> {
        self.add_buyer_locations_impl(&locations).await?;
        Ok(())
    }
    async fn add_landings(
        &self,
        landings: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::Landing>>,
        data_year: u32,
    ) -> CoreResult<()> {
        self.add_landings_impl(landings, data_year).await?;
        Ok(())
    }
    async fn add_ers_dca(
        &self,
        ers_dca: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsDca>>,
    ) -> CoreResult<()> {
        self.add_ers_dca_impl(ers_dca).await?;
        Ok(())
    }
    async fn add_ers_dep(
        &self,
        ers_dep: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsDep>>,
    ) -> CoreResult<()> {
        Ok(self.add_ers_dep_impl(ers_dep).await?)
    }
    async fn add_ers_por(
        &self,
        ers_por: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsPor>>,
    ) -> CoreResult<()> {
        Ok(self.add_ers_por_impl(ers_por).await?)
    }
    async fn add_ers_tra(
        &self,
        ers_tra: BoxIterator<fiskeridir_rs::Result<fiskeridir_rs::ErsTra>>,
    ) -> CoreResult<()> {
        Ok(self.add_ers_tra_impl(ers_tra).await?)
    }
    async fn add_vms(&self, vms: Vec<fiskeridir_rs::Vms>) -> CoreResult<()> {
        Ok(self.add_vms_impl(vms).await?)
    }
    async fn add_aqua_culture_register(
        &self,
        entries: Vec<fiskeridir_rs::AquaCultureEntry>,
    ) -> CoreResult<()> {
        self.add_aqua_culture_register_impl(entries).await?;
        Ok(())
    }
    async fn add_mattilsynet_delivery_points(
        &self,
        delivery_points: Vec<MattilsynetDeliveryPoint>,
    ) -> CoreResult<()> {
        self.add_mattilsynet_delivery_points_impl(delivery_points)
            .await?;
        Ok(())
    }
    async fn add_weather(&self, weather: Vec<NewWeather>) -> CoreResult<()> {
        self.add_weather_impl(weather).await?;
        Ok(())
    }
    async fn add_ocean_climate(&self, ocean_climate: Vec<NewOceanClimate>) -> CoreResult<()> {
        self.add_ocean_climate_impl(ocean_climate).await?;
        Ok(())
    }
}

#[async_trait]
impl ScraperOutboundPort for PostgresAdapter {
    async fn latest_fishing_facility_update(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> CoreResult<Option<DateTime<Utc>>> {
        Ok(self.latest_fishing_facility_update_impl(source).await?)
    }
    async fn latest_weather_timestamp(&self) -> CoreResult<Option<DateTime<Utc>>> {
        Ok(self.latest_weather_timestamp_impl().await?)
    }
    async fn latest_ocean_climate_timestamp(&self) -> CoreResult<Option<DateTime<Utc>>> {
        Ok(self.latest_ocean_climate_timestamp_impl().await?)
    }
    async fn latest_buyer_location_update(&self) -> CoreResult<Option<NaiveDateTime>> {
        Ok(self.latest_buyer_location_update_impl().await?)
    }
    async fn latest_weekly_sale(&self) -> CoreResult<Option<NaiveDate>> {
        Ok(self.latest_weekly_sale_impl().await?)
    }
}

#[async_trait]
impl ScraperFileHashInboundPort for PostgresAdapter {
    async fn add(&self, id: &DataFileId, hash: String) -> CoreResult<()> {
        Ok(self.add_hash(id, hash).await?)
    }
}

#[async_trait]
impl ScraperFileHashOutboundPort for PostgresAdapter {
    async fn get_hashes(&self, ids: &[DataFileId]) -> CoreResult<Vec<(DataFileId, String)>> {
        Ok(self.get_hashes_impl(ids).await?)
    }
}

#[async_trait]
impl TripAssemblerOutboundPort for PostgresAdapter {
    async fn ports(&self) -> CoreResult<Vec<Port>> {
        self.ports_impl().try_convert_collect().await
    }

    async fn dock_points(&self) -> CoreResult<Vec<PortDockPoint>> {
        Ok(self.dock_points_impl().await?)
    }

    async fn all_vessels(&self) -> CoreResult<Vec<Vessel>> {
        self.fiskeridir_ais_vessel_combinations()
            .try_convert_collect()
            .await
    }
    async fn trip_calculation_timer(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> CoreResult<Option<TripCalculationTimer>> {
        Ok(self
            .trip_calculation_timer_impl(vessel_id, trip_assembler_id)
            .await?
            .map(TripCalculationTimer::from))
    }
    async fn trip_prior_to_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        search_timestamp: TripSearchTimestamp,
        trip_assembler: TripAssemblerId,
    ) -> CoreResult<Option<TripAndSucceedingEvents>> {
        let trip = match trip_assembler {
            TripAssemblerId::Landings => self
                .trip_prior_to_timestamp_landings(vessel_id, search_timestamp)
                .await?
                .map(TryFrom::try_from)
                .transpose()?,
            TripAssemblerId::Ers => self
                .trip_prior_to_timestamp_ers(vessel_id, search_timestamp)
                .await?
                .map(TryFrom::try_from)
                .transpose()?,
        };
        Ok(trip)
    }
    async fn all_vessel_events(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler: TripAssemblerId,
    ) -> CoreResult<Vec<VesselEventDetailed>> {
        match trip_assembler {
            TripAssemblerId::Landings => {
                self.all_landing_events(vessel_id)
                    .try_convert_collect()
                    .await
            }
            TripAssemblerId::Ers => {
                self.all_ers_por_and_dep_events(vessel_id)
                    .try_convert_collect()
                    .await
            }
        }
    }
}

#[async_trait]
impl TripPrecisionOutboundPort for PostgresAdapter {
    async fn trip_positions_with_inside_haul(
        &self,
        trip_id: TripId,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        self.trip_positions_with_inside_haul_impl(trip_id, AisPermission::All)
            .convert_collect()
            .await
    }
    async fn departure_weights_from_range(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> CoreResult<Vec<DepartureWeight>> {
        Ok(self
            .departure_weights_from_range_impl(vessel_id, range)
            .await?)
    }
    async fn haul_weights_from_range(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> CoreResult<Vec<HaulWeight>> {
        Ok(self.haul_weights_from_range_impl(vessel_id, range).await?)
    }
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All)
            .convert_collect()
            .await
    }
    async fn ais_vms_positions_with_inside_haul(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        self.ais_vms_positions_with_inside_haul_impl(vessel_id, mmsi, call_sign, range)
            .convert_collect()
            .await
    }
    async fn delivery_points_associated_with_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_landing_coverage: &DateRange,
    ) -> CoreResult<Vec<DeliveryPoint>> {
        Ok(self
            .delivery_points_associated_with_trip_impl(vessel_id, trip_landing_coverage)
            .await?)
    }
    async fn vessel_max_cargo_weight(&self, vessel_id: FiskeridirVesselId) -> CoreResult<f64> {
        Ok(retry(|| self.vessel_max_cargo_weight_impl(vessel_id)).await?)
    }
}

#[async_trait]
impl TripBenchmarkOutbound for PostgresAdapter {
    async fn trips_to_benchmark(&self) -> CoreResult<Vec<BenchmarkTrip>> {
        Ok(retry(|| self.trips_to_benchmark_impl()).await?)
    }
    async fn vessels(&self) -> CoreResult<Vec<Vessel>> {
        retry(|| {
            self.fiskeridir_ais_vessel_combinations()
                .try_convert_collect()
        })
        .await
    }

    async fn overlapping_measurment_fuel(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> CoreResult<f64> {
        Ok(retry(|| self.overlapping_measurment_fuel_impl(vessel_id, range)).await?)
    }

    async fn trip_positions_with_manual(
        &self,
        trip_id: TripId,
    ) -> CoreResult<Vec<TripPositionWithManual>> {
        Ok(retry(|| self.trip_positions_with_manual_impl(trip_id)).await?)
    }
}

#[async_trait]
impl TripBenchmarkInbound for PostgresAdapter {
    async fn add_output(&self, values: &[TripBenchmarkOutput]) -> CoreResult<()> {
        Ok(retry(|| self.add_benchmark_outputs(values)).await?)
    }
}

#[async_trait]
impl HaulDistributorInbound for PostgresAdapter {
    async fn add_output(&self, values: Vec<HaulDistributionOutput>) -> CoreResult<()> {
        Ok(self.add_haul_distribution_output(values).await?)
    }
    async fn update_bycatch_status(&self) -> CoreResult<()> {
        Ok(self.update_bycatch_status_impl().await?)
    }
}

#[async_trait]
impl HaulDistributorOutbound for PostgresAdapter {
    async fn vessels(&self) -> CoreResult<Vec<Vessel>> {
        self.fiskeridir_ais_vessel_combinations()
            .try_convert_collect()
            .await
    }

    async fn catch_locations(&self) -> CoreResult<Vec<CatchLocation>> {
        self.catch_locations_impl(WeatherLocationOverlap::All)
            .try_convert_collect()
            .await
    }

    async fn haul_messages_of_vessel(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<HaulMessage>> {
        Ok(self.haul_messages_of_vessel_impl(vessel_id).await?)
    }

    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All)
            .convert_collect()
            .await
    }
}

#[async_trait]
impl TripPipelineOutbound for PostgresAdapter {
    async fn trips_without_position_cargo_weight_distribution(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>> {
        self.trips_without_position_cargo_weight_distribution_impl(vessel_id)
            .convert_collect()
            .await
    }
    async fn trips_without_position_fuel_consumption_distribution(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>> {
        self.trips_without_position_fuel_consumption_distribution_impl(vessel_id)
            .convert_collect()
            .await
    }
    async fn trips_without_position_layers(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>> {
        self.trips_without_trip_layers_impl(vessel_id)
            .convert_collect()
            .await
    }
    async fn trips_without_distance(&self, vessel_id: FiskeridirVesselId) -> CoreResult<Vec<Trip>> {
        self.trips_without_distance_impl(vessel_id)
            .convert_collect()
            .await
    }
    async fn trips_without_precision(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>> {
        self.trips_without_precision_impl(vessel_id)
            .convert_collect()
            .await
    }
}

#[async_trait]
impl TripPipelineInbound for PostgresAdapter {
    async fn check_for_out_of_order_vms_insertion(&self) -> CoreResult<()> {
        self.check_for_out_of_order_vms_insertion_impl().await?;
        Ok(())
    }
    async fn update_preferred_trip_assemblers(&self) -> CoreResult<()> {
        self.update_preferred_trip_assemblers_impl().await?;
        Ok(())
    }
    async fn update_trip(&self, update: TripUpdate) -> CoreResult<()> {
        self.update_trip_impl(update).await?;
        Ok(())
    }
    async fn add_trip_set(&self, value: TripSet) -> CoreResult<()> {
        self.add_trip_set_impl(value).await?;
        Ok(())
    }
    async fn refresh_detailed_trips(&self, vessel_id: FiskeridirVesselId) -> CoreResult<()> {
        self.refresh_detailed_trips_impl(vessel_id).await?;
        Ok(())
    }

    async fn set_current_trip(&self, vessel_id: FiskeridirVesselId) -> CoreResult<()> {
        let mut tx = self.pool.begin().await.map_err(Error::from)?;
        self.set_current_trip_impl(vessel_id, &mut tx).await?;
        tx.commit().await.map_err(Error::from)?;

        Ok(())
    }
    async fn nuke_trips(&self, vessel_id: FiskeridirVesselId) -> CoreResult<()> {
        self.nuke_trips_impl(vessel_id).await?;
        Ok(())
    }
}

#[async_trait]
impl MatrixCacheVersion for PostgresAdapter {
    async fn increment(&self) -> CoreResult<()> {
        self.increment_duckdb_version().await?;
        Ok(())
    }
}

#[async_trait]
impl VerificationOutbound for PostgresAdapter {
    async fn verify_database(&self) -> CoreResult<()> {
        self.verify_database_impl().await?;
        Ok(())
    }
}

#[async_trait]
impl MLModelsOutbound for PostgresAdapter {
    async fn catch_locations_weather_dates(
        &self,
        dates: Vec<NaiveDate>,
    ) -> CoreResult<Vec<CatchLocationWeather>> {
        Ok(self.catch_locations_weather_dates_impl(dates).await?)
    }

    async fn catch_locations_weather(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> CoreResult<Vec<CatchLocationWeather>> {
        Ok(self.catch_locations_weather_impl(keys).await?)
    }

    async fn save_model(
        &self,
        model_id: ModelId,
        model: &[u8],
        species: SpeciesGroup,
    ) -> CoreResult<()> {
        self.save_model_impl(model_id, model, species).await?;
        Ok(())
    }
    async fn catch_locations(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> CoreResult<Vec<CatchLocation>> {
        self.catch_locations_impl(overlap)
            .try_convert_collect()
            .await
    }

    async fn model(&self, model_id: ModelId, species: SpeciesGroup) -> CoreResult<Vec<u8>> {
        Ok(self.model_impl(model_id, species).await?)
    }
    async fn fishing_weight_predictor_training_data(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        weather_data: WeatherData,
        limit: Option<u32>,
        bycatch_percentage: Option<f64>,
        majority_species_group: bool,
    ) -> CoreResult<Vec<WeightPredictorTrainingData>> {
        self.fishing_weight_predictor_training_data_impl(
            model_id,
            species,
            weather_data,
            limit,
            bycatch_percentage,
            majority_species_group,
        )
        .convert_collect()
        .await
    }

    async fn fishing_spot_predictor_training_data(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        limit: Option<u32>,
    ) -> CoreResult<Vec<FishingSpotTrainingData>> {
        Ok(self
            .fishing_spot_predictor_training_data_impl(model_id, species, limit)
            .await?)
    }

    async fn commit_hauls_training(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        hauls: Vec<TrainingHaul>,
    ) -> CoreResult<()> {
        self.commit_hauls_training_impl(model_id, species, hauls)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl MLModelsInbound for PostgresAdapter {
    async fn catch_locations_weather_dates(
        &self,
        dates: Vec<NaiveDate>,
    ) -> CoreResult<Vec<CatchLocationWeather>> {
        Ok(self.catch_locations_weather_dates_impl(dates).await?)
    }

    async fn catch_locations_weather(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> CoreResult<Vec<CatchLocationWeather>> {
        Ok(self.catch_locations_weather_impl(keys).await?)
    }

    async fn existing_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> CoreResult<Vec<FishingWeightPrediction>> {
        Ok(self
            .existing_fishing_weight_predictions_impl(model_id, species, year)
            .await?)
    }
    async fn existing_fishing_spot_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> CoreResult<Vec<FishingSpotPrediction>> {
        Ok(self
            .existing_fishing_spot_predictions_impl(model_id, species, year)
            .await?)
    }
    async fn catch_locations(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> CoreResult<Vec<CatchLocation>> {
        self.catch_locations_impl(overlap)
            .try_convert_collect()
            .await
    }
    async fn add_fishing_spot_predictions(
        &self,
        predictions: Vec<NewFishingSpotPrediction>,
    ) -> CoreResult<()> {
        self.add_fishing_spot_predictions_impl(predictions).await?;
        Ok(())
    }
    async fn add_fishing_weight_predictions(
        &self,
        predictions: Vec<NewFishingWeightPrediction>,
    ) -> CoreResult<()> {
        self.add_weight_predictions_impl(predictions).await?;
        Ok(())
    }
}

#[async_trait]
impl HaulWeatherInbound for PostgresAdapter {
    async fn add_haul_weather(&self, values: Vec<HaulWeatherOutput>) -> CoreResult<()> {
        self.add_haul_weather_impl(values).await?;
        Ok(())
    }
}

#[async_trait]
impl HaulWeatherOutbound for PostgresAdapter {
    async fn all_vessels(&self) -> CoreResult<Vec<Vessel>> {
        self.fiskeridir_ais_vessel_combinations()
            .try_convert_collect()
            .await
    }
    async fn haul_messages_of_vessel_without_weather(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<HaulMessage>> {
        Ok(self
            .haul_messages_of_vessel_without_weather_impl(vessel_id)
            .await?)
    }
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All)
            .convert_collect()
            .await
    }
    async fn weather_locations(&self) -> CoreResult<Vec<WeatherLocation>> {
        self.weather_locations_impl().try_convert_collect().await
    }
    async fn haul_weather(&self, query: WeatherQuery) -> CoreResult<Option<HaulWeather>> {
        Ok(self.haul_weather_impl(query).await?)
    }
    async fn haul_ocean_climate(
        &self,
        query: OceanClimateQuery,
    ) -> CoreResult<Option<HaulOceanClimate>> {
        Ok(self.haul_ocean_climate_impl(query).await?)
    }
}

#[async_trait]
impl MeilisearchSource for PostgresAdapter {
    async fn trips_by_ids(&self, trip_ids: &[TripId]) -> CoreResult<Vec<TripDetailed>> {
        self.detailed_trips_by_ids_impl(trip_ids)
            .try_convert_collect()
            .await
    }
    async fn hauls_by_ids(&self, haul_ids: &[HaulId]) -> CoreResult<Vec<Haul>> {
        self.hauls_by_ids_impl(haul_ids).try_convert_collect().await
    }
    async fn landings_by_ids(&self, landing_ids: &[LandingId]) -> CoreResult<Vec<Landing>> {
        self.landings_by_ids_impl(landing_ids)
            .try_convert_collect()
            .await
    }
    async fn all_trip_versions(&self) -> CoreResult<Vec<(TripId, i64)>> {
        Ok(self.all_trip_cache_versions_impl().await?)
    }
    async fn all_haul_versions(&self) -> CoreResult<Vec<(HaulId, i64)>> {
        Ok(self.all_haul_cache_versions_impl().await?)
    }
    async fn all_landing_versions(&self) -> CoreResult<Vec<(LandingId, i64)>> {
        Ok(self.all_landing_versions_impl().await?)
    }
}

#[async_trait]
pub(crate) trait Convert<T, E> {
    fn convert(self) -> impl Stream<Item = StdResult<T, E>>;
    async fn convert_collect(self) -> StdResult<Vec<T>, E>;
}

#[async_trait]
pub(crate) trait TryConvert<T, E> {
    fn try_convert(self) -> impl Stream<Item = StdResult<T, E>>;
    async fn try_convert_collect(self) -> StdResult<Vec<T>, E>;
}

#[async_trait]
impl<S, T, O, E> Convert<O, E> for S
where
    S: Stream<Item = Result<T>> + Send,
    O: From<T> + Send,
    E: From<Error>,
{
    fn convert(self) -> impl Stream<Item = StdResult<O, E>> {
        self.map_ok(From::from).map_err(|e| e.into())
    }
    async fn convert_collect(self) -> StdResult<Vec<O>, E> {
        self.convert().try_collect().await
    }
}

#[async_trait]
impl<S, T, O, E> TryConvert<O, E> for S
where
    S: Stream<Item = Result<T>> + Send,
    O: TryFrom<T, Error = Error> + Send,
    E: From<Error>,
{
    fn try_convert(self) -> impl Stream<Item = StdResult<O, E>> {
        self.map(|v| Ok(v?.try_into()?))
    }
    async fn try_convert_collect(self) -> StdResult<Vec<O>, E> {
        self.try_convert().try_collect().await
    }
}

pub(crate) fn convert_optional<A, B, E>(val: Option<A>) -> std::result::Result<Option<B>, E>
where
    B: std::convert::TryFrom<A, Error = crate::error::Error>,
    E: std::convert::From<crate::error::Error>,
{
    Ok(val.map(B::try_from).transpose()?)
}

// When our managed azure postgres db encounters a failover to its replica our
// 'ais-consumer' and 'processeors' services becomes idle(?) and stops producing tracing spans.
// We suspect this is because they are still connected to the replica when the main db node
// comes back up and the replica becomes read-only rendering the connections to the replica as read-only.
// According to the provided source this fix *MIGHT* resolve our issue.
//
// Fix source: https://github.com/launchbadge/sqlx/issues/2290#issuecomment-1663353872
async fn check_if_read_only_connection(
    conn: &mut PgConnection,
) -> std::result::Result<(), sqlx::Error> {
    let sql = "show transaction_read_only";
    let transaction_read_only: String = sqlx::query_scalar(sql).fetch_one(conn).await?;
    if transaction_read_only == "on" {
        Err(sqlx::Error::Configuration(
            "Attempted to connect to read-only database,
                        most likely a replica which is probably caused by a database failover"
                .into(),
        ))
    } else {
        Ok(())
    }
}
