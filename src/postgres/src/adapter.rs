use crate::error::Result;
use crate::models::LandingMatrixArgs;
use crate::queries::haul::HaulsMatrixArgs;
use crate::{ers_dep_set::ErsDepSet, ers_por_set::ErsPorSet, ers_tra_set::ErsTraSet};
use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::{CallSign, DataFileId, DeliveryPointId, LandingId, SpeciesGroup};
use futures::{Stream, StreamExt, TryStreamExt};
use kyogre_core::*;
use orca_core::{Environment, PsqlLogStatements, PsqlSettings};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use std::{pin::Pin, result::Result as StdResult};
use tracing::{error, instrument, warn};

#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
    ais_pool: Option<PgPool>,
    pub(crate) ignored_conflict_call_signs: Vec<String>,
    pub(crate) environment: Environment,
}

enum ConsumeLoopOutcome {
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

        let ais_pool = match environment {
            Environment::Development | Environment::Production | Environment::Local => {
                let ais_opts = opts
                    .clone()
                    .options([("plan_cache_mode", "force_custom_plan")]);

                let ais_pool = PgPoolOptions::new()
                    .min_connections(1)
                    .max_connections(connections_per_pool)
                    .acquire_timeout(std::time::Duration::from_secs(60))
                    .connect_with(ais_opts)
                    .await?;

                Some(ais_pool)
            }
            Environment::OnPremise | Environment::Test => None,
        };

        let pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .connect_with(opts)
            .await?;

        let ignored_conflict_call_signs: Vec<String> = IGNORED_CONFLICT_CALL_SIGNS
            .iter()
            .map(|v| v.to_string())
            .collect();

        Ok(PostgresAdapter {
            pool,
            ais_pool,
            ignored_conflict_call_signs,
            environment,
        })
    }

    pub async fn close(self) {
        self.pool.close().await
    }

    pub(crate) fn ais_pool(&self) -> &PgPool {
        match self.environment {
            Environment::Production
            | Environment::Development
            | Environment::OnPremise
            | Environment::Local => self.ais_pool.as_ref().unwrap(),
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

    pub async fn do_migrations(&self) {
        sqlx::migrate!()
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .unwrap();
    }

    #[instrument(skip_all)]
    pub async fn consume_loop_iteration(
        &self,
        receiver: &mut tokio::sync::broadcast::Receiver<DataMessage>,
        process_confirmation: Option<&tokio::sync::mpsc::Sender<()>>,
    ) -> ConsumeLoopOutcome {
        let message = receiver.recv().await;
        let result = self.process_message(message).await;
        // Only enabled in tests
        if let Some(s) = process_confirmation {
            s.send(()).await.unwrap();
        }
        match result {
            AisProcessingAction::Exit => ConsumeLoopOutcome::Exit,
            AisProcessingAction::Continue => ConsumeLoopOutcome::Continue,
            AisProcessingAction::Retry {
                positions,
                static_messages,
            } => {
                let mut err = Ok(());
                for _ in 0..2 {
                    err = self
                        .insertion_retry(&mut positions.as_deref(), &mut static_messages.as_deref())
                        .await;

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
        mut receiver: tokio::sync::broadcast::Receiver<DataMessage>,
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

    #[instrument(skip_all, name = "postgres_insert_ais_data")]
    async fn process_message(
        &self,
        incoming: StdResult<DataMessage, tokio::sync::broadcast::error::RecvError>,
    ) -> AisProcessingAction {
        match incoming {
            Ok(message) => {
                match (
                    self.add_ais_positions(&message.positions).await,
                    self.add_ais_vessels(&message.static_messages).await,
                ) {
                    (Ok(_), Ok(_)) => AisProcessingAction::Continue,
                    (Ok(_), Err(_)) => AisProcessingAction::Retry {
                        positions: None,
                        static_messages: Some(message.static_messages),
                    },
                    (Err(_), Ok(_)) => AisProcessingAction::Retry {
                        positions: Some(message.positions),
                        static_messages: None,
                    },
                    (Err(_), Err(_)) => AisProcessingAction::Retry {
                        positions: Some(message.positions),
                        static_messages: Some(message.static_messages),
                    },
                }
            }
            Err(e) => match e {
                tokio::sync::broadcast::error::RecvError::Closed => {
                    warn!("sender half of ais broadcast channel closed unexpectedly, exiting");
                    AisProcessingAction::Exit
                }
                tokio::sync::broadcast::error::RecvError::Lagged(num_lagged) => {
                    warn!("postgres consumer lagged {num_lagged} ais messages");
                    AisProcessingAction::Continue
                }
            },
        }
    }
}

impl TestStorage for PostgresAdapter {}

#[async_trait]
impl TestHelperOutbound for PostgresAdapter {
    async fn trip_assembler_log(&self) -> Vec<TripAssemblerLogEntry> {
        self.trip_assembler_log_impl()
            .await
            .unwrap()
            .into_iter()
            .map(TripAssemblerLogEntry::try_from)
            .collect::<StdResult<_, _>>()
            .unwrap()
    }
    async fn active_vessel_conflicts(&self) -> Vec<ActiveVesselConflict> {
        self.active_vessel_conflicts_impl()
            .await
            .unwrap()
            .into_iter()
            .map(ActiveVesselConflict::try_from)
            .collect::<std::result::Result<_, _>>()
            .unwrap()
    }
    async fn all_dep(&self) -> Vec<Departure> {
        self.all_ers_departures_impl()
            .await
            .unwrap()
            .into_iter()
            .map(kyogre_core::Departure::from)
            .collect()
    }
    async fn all_por(&self) -> Vec<Arrival> {
        self.all_ers_arrivals_impl()
            .await
            .unwrap()
            .into_iter()
            .map(kyogre_core::Arrival::from)
            .collect()
    }

    async fn delivery_points_log(&self) -> Vec<serde_json::Value> {
        self.delivery_points_log_impl().await.unwrap()
    }

    async fn all_ais(&self) -> Vec<AisPosition> {
        self.all_ais_impl().await.unwrap()
    }
    async fn all_vms(&self) -> Vec<VmsPosition> {
        self.all_vms_impl()
            .await
            .unwrap()
            .into_iter()
            .map(VmsPosition::try_from)
            .collect::<StdResult<_, _>>()
            .unwrap()
    }
    async fn all_ais_vms(&self) -> Vec<AisVmsPosition> {
        self.all_ais_vms_impl()
            .await
            .unwrap()
            .into_iter()
            .map(AisVmsPosition::try_from)
            .collect::<StdResult<_, _>>()
            .unwrap()
    }
    async fn port(&self, port_id: &str) -> Option<Port> {
        self.port_impl(port_id)
            .await
            .unwrap()
            .map(Port::try_from)
            .transpose()
            .unwrap()
    }
    async fn delivery_point(&self, id: &DeliveryPointId) -> Option<DeliveryPoint> {
        self.delivery_point_impl(id)
            .await
            .unwrap()
            .map(DeliveryPoint::try_from)
            .transpose()
            .unwrap()
    }
    async fn dock_points_of_port(&self, port_id: &str) -> Vec<PortDockPoint> {
        self.dock_points_of_port_impl(port_id)
            .await
            .unwrap()
            .into_iter()
            .map(PortDockPoint::try_from)
            .collect::<StdResult<_, _>>()
            .unwrap()
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
        self.add_manual_delivery_points_impl(
            values
                .into_iter()
                .map(crate::models::ManualDeliveryPoint::from)
                .collect(),
        )
        .await
        .unwrap();
    }
    async fn add_deprecated_delivery_point(
        &self,
        old: DeliveryPointId,
        new: DeliveryPointId,
    ) -> CoreResult<()> {
        Ok(self.add_deprecated_delivery_point_impl(old, new).await?)
    }
}

#[async_trait]
impl AisConsumeLoop for PostgresAdapter {
    async fn consume(
        &self,
        receiver: tokio::sync::broadcast::Receiver<DataMessage>,
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
        Ok(self.all_ais_positions_impl(mmsi, start, end).await?)
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
impl WebApiOutboundPort for PostgresAdapter {
    fn ais_current_positions(
        &self,
        limit: Option<DateTime<Utc>>,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisPosition> {
        self.ais_current_positions(limit, user_policy)
            .map_err(From::from)
            .boxed()
    }
    fn ais_vms_area_positions(
        &self,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        date_limit: NaiveDate,
    ) -> PinBoxStream<'_, AisVmsAreaCount> {
        self.ais_vms_area_positions_impl(x1, x2, y1, y2, date_limit)
            .map(|v| Ok(v?))
            .boxed()
    }
    fn fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
        limit: u32,
    ) -> PinBoxStream<'_, FishingWeightPrediction> {
        convert_stream(self.fishing_weight_predictions_impl(model_id, species, date, limit)).boxed()
    }
    fn all_fishing_weight_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingWeightPrediction> {
        convert_stream(self.all_fishing_weight_predictions_impl(model_id)).boxed()
    }

    fn all_fishing_spot_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingSpotPrediction> {
        self.all_fishing_spot_predictions_impl(model_id)
            .map_err(From::from)
            .boxed()
    }
    async fn fishing_spot_prediction(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
    ) -> CoreResult<Option<FishingSpotPrediction>> {
        Ok(self
            .fishing_spot_prediction_impl(model_id, species, date)
            .await?)
    }
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        permission: AisPermission,
    ) -> PinBoxStream<'_, AisPosition> {
        self.ais_positions_impl(mmsi, range, permission)
            .map_err(From::from)
            .boxed()
    }
    fn vms_positions(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> PinBoxStream<'_, VmsPosition> {
        convert_stream(self.vms_positions_impl(call_sign, range)).boxed()
    }

    fn ais_vms_positions(
        &self,
        params: AisVmsParams,
        permission: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition> {
        match params {
            AisVmsParams::Trip(trip_id) => {
                convert_stream(self.trip_positions_impl(trip_id, permission)).boxed()
            }
            AisVmsParams::Range {
                mmsi,
                call_sign,
                range,
            } => convert_stream(self.ais_vms_positions_impl(
                mmsi,
                call_sign.as_ref(),
                &range,
                permission,
            ))
            .boxed(),
        }
    }

    fn species(&self) -> PinBoxStream<'_, Species> {
        convert_stream(self.species_impl()).boxed()
    }

    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir> {
        convert_stream(self.species_fiskeridir_impl()).boxed()
    }
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao> {
        convert_stream(self.species_fao_impl()).boxed()
    }
    fn vessels(&self) -> Pin<Box<dyn Stream<Item = CoreResult<Vessel>> + Send + '_>> {
        convert_stream(self.fiskeridir_ais_vessel_combinations()).boxed()
    }

    fn hauls(&self, query: HaulsQuery) -> CoreResult<PinBoxStream<'_, Haul>> {
        let stream = self.hauls_impl(query)?;
        Ok(convert_stream(stream).boxed())
    }

    async fn detailed_trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>> {
        convert_optional(
            self.detailed_trip_of_haul_impl(haul_id, read_fishing_facility)
                .await?,
        )
    }

    async fn detailed_trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>> {
        convert_optional(
            self.detailed_trip_of_landing_impl(landing_id, read_fishing_facility)
                .await?,
        )
    }

    fn detailed_trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> CoreResult<PinBoxStream<'_, TripDetailed>> {
        let stream = self.detailed_trips_impl(query, read_fishing_facility)?;
        Ok(convert_stream(stream).boxed())
    }

    async fn current_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<CurrentTrip>> {
        convert_optional(
            self.current_trip_impl(vessel_id, read_fishing_facility)
                .await?,
        )
    }

    fn landings(&self, query: LandingsQuery) -> CoreResult<PinBoxStream<'_, Landing>> {
        let stream = self.landings_impl(query)?;
        Ok(convert_stream(stream).boxed())
    }

    async fn landing_matrix(&self, query: &LandingMatrixQuery) -> CoreResult<LandingMatrix> {
        let active_filter = query.active_filter;
        let args = LandingMatrixArgs::try_from(query.clone())?;

        let j1 = tokio::spawn(PostgresAdapter::landing_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            LandingMatrixXFeature::Date,
        ));
        let j2 = tokio::spawn(PostgresAdapter::landing_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            LandingMatrixXFeature::VesselLength,
        ));
        let j3 = tokio::spawn(PostgresAdapter::landing_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            LandingMatrixXFeature::GearGroup,
        ));
        let j4 = tokio::spawn(PostgresAdapter::landing_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            LandingMatrixXFeature::SpeciesGroup,
        ));

        let (dates, length_group, gear_group, species_group) = tokio::join!(j1, j2, j3, j4);

        Ok(LandingMatrix {
            dates: dates??,
            length_group: length_group??,
            gear_group: gear_group??,
            species_group: species_group??,
        })
    }

    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> CoreResult<HaulsMatrix> {
        let active_filter = query.active_filter;
        let args = HaulsMatrixArgs::try_from(query.clone())?;

        let j1 = tokio::spawn(PostgresAdapter::hauls_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::Date,
        ));
        let j2 = tokio::spawn(PostgresAdapter::hauls_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::VesselLength,
        ));
        let j3 = tokio::spawn(PostgresAdapter::hauls_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::GearGroup,
        ));
        let j4 = tokio::spawn(PostgresAdapter::hauls_matrix_impl(
            self.pool.clone(),
            args.clone(),
            active_filter,
            HaulMatrixXFeature::SpeciesGroup,
        ));

        let (dates, length_group, gear_group, species_group) = tokio::join!(j1, j2, j3, j4);

        Ok(HaulsMatrix {
            dates: dates??,
            length_group: length_group??,
            gear_group: gear_group??,
            species_group: species_group??,
        })
    }

    fn fishing_facilities(
        &self,
        query: FishingFacilitiesQuery,
    ) -> PinBoxStream<'_, FishingFacility> {
        convert_stream(self.fishing_facilities_impl(query)).boxed()
    }

    async fn get_user(&self, user_id: BarentswatchUserId) -> CoreResult<Option<User>> {
        convert_optional(self.get_user_impl(user_id).await?)
    }

    fn delivery_points(&self) -> PinBoxStream<'_, DeliveryPoint> {
        convert_stream(self.delivery_points_impl()).boxed()
    }

    fn weather(&self, query: WeatherQuery) -> CoreResult<PinBoxStream<'_, Weather>> {
        let stream = self.weather_impl(query)?;
        Ok(convert_stream(stream).boxed())
    }

    fn weather_locations(&self) -> PinBoxStream<'_, WeatherLocation> {
        convert_stream(self.weather_locations_impl()).boxed()
    }

    fn fuel_measurements(&self, query: FuelMeasurementsQuery) -> PinBoxStream<'_, FuelMeasurement> {
        convert_stream(self.fuel_measurements_impl(query)).boxed()
    }

    async fn vessel_benchmarks(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
    ) -> CoreResult<VesselBenchmarks> {
        Ok(self
            .vessel_benchmarks_impl(user_id, call_sign)
            .await?
            .try_into()?)
    }
}

#[async_trait]
impl WebApiInboundPort for PostgresAdapter {
    async fn update_user(&self, user: User) -> CoreResult<()> {
        self.update_user_impl(user).await?;
        Ok(())
    }
    async fn add_fuel_measurements(&self, measurements: Vec<FuelMeasurement>) -> CoreResult<()> {
        self.add_fuel_measurements_impl(measurements).await?;
        Ok(())
    }
    async fn update_fuel_measurements(&self, measurements: Vec<FuelMeasurement>) -> CoreResult<()> {
        self.update_fuel_measurements_impl(measurements).await?;
        Ok(())
    }
    async fn delete_fuel_measurements(
        &self,
        measurements: Vec<DeleteFuelMeasurement>,
    ) -> CoreResult<()> {
        self.delete_fuel_measurements_impl(measurements).await?;
        Ok(())
    }
}

#[async_trait]
impl AisVmsAreaPrunerInbound for PostgresAdapter {
    async fn prune_ais_vms_area(&self, limit: NaiveDate) -> CoreResult<()> {
        Ok(self.prune_ais_vms_area_impl(limit).await?)
    }
}

#[async_trait]
impl ScraperInboundPort for PostgresAdapter {
    async fn add_fishing_facilities(&self, facilities: Vec<FishingFacility>) -> CoreResult<()> {
        self.add_fishing_facilities_impl(facilities).await?;
        Ok(())
    }
    async fn add_register_vessels(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> CoreResult<()> {
        self.add_register_vessels_full(vessels).await?;
        Ok(())
    }
    async fn add_landings(
        &self,
        landings: Box<
            dyn Iterator<Item = std::result::Result<fiskeridir_rs::Landing, fiskeridir_rs::Error>>
                + Send
                + Sync,
        >,
        data_year: u32,
    ) -> CoreResult<()> {
        self.add_landings_impl(landings, data_year).await?;
        Ok(())
    }
    async fn add_ers_dca(
        &self,
        ers_dca: Box<
            dyn Iterator<Item = std::result::Result<fiskeridir_rs::ErsDca, fiskeridir_rs::Error>>
                + Send
                + Sync,
        >,
    ) -> CoreResult<()> {
        self.add_ers_dca_impl(ers_dca).await?;
        Ok(())
    }
    async fn add_ers_dep(&self, ers_dep: Vec<fiskeridir_rs::ErsDep>) -> CoreResult<()> {
        let set = ErsDepSet::new(ers_dep)?;
        Ok(self.add_ers_dep_set(set).await?)
    }
    async fn add_ers_por(&self, ers_por: Vec<fiskeridir_rs::ErsPor>) -> CoreResult<()> {
        let set = ErsPorSet::new(ers_por)?;
        Ok(self.add_ers_por_set(set).await?)
    }
    async fn add_ers_tra(&self, ers_tra: Vec<fiskeridir_rs::ErsTra>) -> CoreResult<()> {
        let set = ErsTraSet::new(ers_tra)?;
        Ok(self.add_ers_tra_set(set).await?)
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
        convert_vec(self.ports_impl().await?)
    }

    async fn dock_points(&self) -> CoreResult<Vec<PortDockPoint>> {
        convert_vec(self.dock_points_impl().await?)
    }

    async fn all_vessels(&self) -> CoreResult<Vec<Vessel>> {
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
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
        timestamp: &DateTime<Utc>,
        bound: Bound,
    ) -> CoreResult<Option<Trip>> {
        convert_optional(match bound {
            Bound::Inclusive => {
                self.trip_prior_to_timestamp_inclusive(vessel_id, timestamp)
                    .await?
            }
            Bound::Exclusive => {
                self.trip_prior_to_timestamp_exclusive(vessel_id, timestamp)
                    .await?
            }
        })
    }
    async fn relevant_events(
        &self,
        vessel_id: FiskeridirVesselId,
        period: &QueryRange,
        event_types: RelevantEventType,
    ) -> CoreResult<Vec<VesselEventDetailed>> {
        convert_vec(match event_types {
            RelevantEventType::Landing => self.landing_events(vessel_id, period).await,
            RelevantEventType::ErsPorAndDep => self.ers_por_and_dep_events(vessel_id, period).await,
        }?)
    }
}

#[async_trait]
impl TripPrecisionOutboundPort for PostgresAdapter {
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All))
            .try_collect()
            .await
    }
    async fn delivery_points_associated_with_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_landing_coverage: &DateRange,
    ) -> CoreResult<Vec<DeliveryPoint>> {
        convert_vec(
            self.delivery_points_associated_with_trip_impl(vessel_id, trip_landing_coverage)
                .await?,
        )
    }
}

#[async_trait]
impl VesselBenchmarkOutbound for PostgresAdapter {
    async fn vessels(&self) -> CoreResult<Vec<Vessel>> {
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }
    async fn sum_trip_time(&self, id: FiskeridirVesselId) -> CoreResult<Option<Duration>> {
        Ok(self.sum_trip_time_impl(id).await?)
    }
    async fn sum_landing_weight(&self, id: FiskeridirVesselId) -> CoreResult<Option<f64>> {
        Ok(self.sum_landing_weight_impl(id).await?)
    }
}

#[async_trait]
impl VesselBenchmarkInbound for PostgresAdapter {
    async fn add_output(&self, values: Vec<VesselBenchmarkOutput>) -> CoreResult<()> {
        Ok(self.add_benchmark_outputs(values).await?)
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
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }

    async fn catch_locations(&self) -> CoreResult<Vec<CatchLocation>> {
        convert_vec(
            self.catch_locations_impl(WeatherLocationOverlap::All)
                .await?,
        )
    }

    async fn haul_messages_of_vessel(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<HaulMessage>> {
        convert_vec(self.haul_messages_of_vessel_impl(vessel_id).await?)
    }

    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All))
            .try_collect()
            .await
    }
}

#[async_trait]
impl TripPipelineOutbound for PostgresAdapter {
    async fn trips_without_position_layers(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>> {
        convert_vec(self.trips_without_trip_layers_impl(vessel_id).await?)
    }
    async fn trips_without_distance(&self, vessel_id: FiskeridirVesselId) -> CoreResult<Vec<Trip>> {
        convert_vec(self.trips_without_distance_impl(vessel_id).await?)
    }
    async fn trips_without_precision(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>> {
        convert_vec(self.trips_without_precision_impl(vessel_id).await?)
    }
}

#[async_trait]
impl TripPipelineInbound for PostgresAdapter {
    async fn reset_trip_processing_conflicts(&self) -> CoreResult<()> {
        self.reset_trip_processing_conflicts_impl().await?;
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
        convert_vec(self.catch_locations_weather_dates_impl(dates).await?)
    }

    async fn catch_locations_weather(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> CoreResult<Vec<CatchLocationWeather>> {
        convert_vec(self.catch_locations_weather_impl(keys).await?)
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
        convert_vec(self.catch_locations_impl(overlap).await?)
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
        Ok(self
            .fishing_weight_predictor_training_data_impl(
                model_id,
                species,
                weather_data,
                limit,
                bycatch_percentage,
                majority_species_group,
            )
            .await?
            .into_iter()
            .map(WeightPredictorTrainingData::from)
            .collect())
    }

    async fn fishing_spot_predictor_training_data(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        limit: Option<u32>,
    ) -> CoreResult<Vec<FishingSpotTrainingData>> {
        convert_vec(
            self.fishing_spot_predictor_training_data_impl(model_id, species, limit)
                .await?,
        )
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
        convert_vec(self.catch_locations_weather_dates_impl(dates).await?)
    }

    async fn catch_locations_weather(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> CoreResult<Vec<CatchLocationWeather>> {
        convert_vec(self.catch_locations_weather_impl(keys).await?)
    }

    async fn existing_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        year: u32,
    ) -> CoreResult<Vec<FishingWeightPrediction>> {
        convert_vec(
            self.existing_fishing_weight_predictions_impl(model_id, species, year)
                .await?,
        )
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
        convert_vec(self.catch_locations_impl(overlap).await?)
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
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }
    async fn haul_messages_of_vessel_without_weather(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<HaulMessage>> {
        convert_vec(
            self.haul_messages_of_vessel_without_weather_impl(vessel_id)
                .await?,
        )
    }
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All))
            .try_collect()
            .await
    }
    async fn weather_locations(&self) -> CoreResult<Vec<WeatherLocation>> {
        convert_stream(self.weather_locations_impl())
            .try_collect()
            .await
    }
    async fn haul_weather(&self, query: WeatherQuery) -> CoreResult<Option<HaulWeather>> {
        convert_optional(self.haul_weather_impl(query).await?)
    }
    async fn haul_ocean_climate(
        &self,
        query: OceanClimateQuery,
    ) -> CoreResult<Option<HaulOceanClimate>> {
        convert_optional(self.haul_ocean_climate_impl(query).await?)
    }
}

#[async_trait]
impl MeilisearchSource for PostgresAdapter {
    async fn trips_by_ids(&self, trip_ids: &[TripId]) -> CoreResult<Vec<TripDetailed>> {
        convert_vec(self.detailed_trips_by_ids_impl(trip_ids).await?)
    }
    async fn hauls_by_ids(&self, haul_ids: &[HaulId]) -> CoreResult<Vec<Haul>> {
        convert_vec(self.hauls_by_ids_impl(haul_ids).await?)
    }
    async fn landings_by_ids(&self, landing_ids: &[LandingId]) -> CoreResult<Vec<Landing>> {
        convert_vec(self.landings_by_ids_impl(landing_ids).await?)
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

pub(crate) fn convert_stream<I, A, B>(input: I) -> impl Stream<Item = CoreResult<B>>
where
    I: Stream<Item = Result<A>>,
    B: TryFrom<A>,
    B: std::convert::TryFrom<A, Error = crate::error::Error>,
{
    input.map(|i| Ok(i.map(B::try_from)??))
}

pub(crate) fn convert_optional<A, B>(val: Option<A>) -> CoreResult<Option<B>>
where
    B: std::convert::TryFrom<A, Error = crate::error::Error>,
{
    Ok(val.map(B::try_from).transpose()?)
}

pub(crate) fn convert_vec<A, B>(val: Vec<A>) -> CoreResult<Vec<B>>
where
    B: std::convert::TryFrom<A, Error = crate::error::Error>,
{
    Ok(val
        .into_iter()
        .map(B::try_from)
        .collect::<std::result::Result<Vec<_>, <B as std::convert::TryFrom<A>>::Error>>()?)
}
