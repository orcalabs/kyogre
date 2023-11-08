use crate::models::LandingMatrixArgs;
use crate::queries::haul::HaulsMatrixArgs;
use crate::{
    error::PostgresError, ers_dep_set::ErsDepSet, ers_por_set::ErsPorSet, ers_tra_set::ErsTraSet,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use error_stack::{Report, Result, ResultExt};
use fiskeridir_rs::{CallSign, DeliveryPointId, LandingId, SpeciesGroup};
use futures::{Stream, StreamExt, TryStreamExt};
use kyogre_core::*;
use orca_core::{Environment, PsqlLogStatements, PsqlSettings};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    ConnectOptions, PgPool,
};
use std::pin::Pin;
use tracing::{event, instrument, Level};

#[derive(Debug, Clone)]
pub struct PostgresAdapter {
    pub(crate) pool: PgPool,
    pub(crate) ais_pool: PgPool,
    pub(crate) ignored_conflict_call_signs: Vec<String>,
    pub(crate) environment: Option<Environment>,
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
    pub async fn new(settings: &PsqlSettings) -> Result<PostgresAdapter, PostgresError> {
        let environment: Option<Environment> = std::env::var("APP_ENVIRONMENT")
            .ok()
            .and_then(|v| v.try_into().ok());

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
                opts = opts.disable_statement_logging();
            }
        }

        let ais_opts = opts
            .clone()
            .options([("plan_cache_mode", "force_custom_plan")]);

        let pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .connect_with(opts)
            .await
            .change_context(PostgresError::Connection)?;

        let ais_pool = PgPoolOptions::new()
            .max_connections(connections_per_pool)
            .acquire_timeout(std::time::Duration::from_secs(60))
            .connect_with(ais_opts)
            .await
            .change_context(PostgresError::Connection)?;

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

    pub async fn do_migrations(&self) {
        sqlx::migrate!()
            .set_ignore_missing(true)
            .run(&self.pool)
            .await
            .unwrap();
    }

    pub async fn consume_loop(
        &self,
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
                    static_messages,
                } => {
                    for _ in 0..2 {
                        self.insertion_retry(positions.as_deref(), static_messages.as_deref())
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
        static_messages: Option<&[NewAisStatic]>,
    ) {
        if let Some(positions) = positions {
            if let Err(e) = self.add_ais_positions(positions).await {
                event!(Level::ERROR, "failed to add ais positions: {:?}", e);
            }
        }

        if let Some(static_messages) = static_messages {
            if let Err(e) = self.add_ais_vessels(static_messages).await {
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
                match (
                    self.add_ais_positions(&message.positions).await,
                    self.add_ais_vessels(&message.static_messages).await,
                ) {
                    (Ok(_), Ok(_)) => AisProcessingAction::Continue,
                    (Ok(_), Err(e)) => {
                        event!(Level::ERROR, "failed to add ais static: {:?}", e);
                        AisProcessingAction::Retry {
                            positions: None,
                            static_messages: Some(message.static_messages),
                        }
                    }
                    (Err(e), Ok(_)) => {
                        event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            static_messages: None,
                        }
                    }
                    (Err(e), Err(e2)) => {
                        event!(Level::ERROR, "failed to add ais positions: {:?}", e);
                        event!(Level::ERROR, "failed to add ais static: {:?}", e2);
                        AisProcessingAction::Retry {
                            positions: Some(message.positions),
                            static_messages: Some(message.static_messages),
                        }
                    }
                }
            }
            Err(e) => match e {
                tokio::sync::broadcast::error::RecvError::Closed => {
                    event!(
                        Level::WARN,
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
            .change_context(PostgresError::Transaction)
    }
}

impl TestStorage for PostgresAdapter {}

#[async_trait]
impl TestHelperOutbound for PostgresAdapter {
    async fn active_vessel_conflicts(&self) -> Vec<ActiveVesselConflict> {
        self.active_vessel_conflicts_impl()
            .await
            .unwrap()
            .into_iter()
            .map(ActiveVesselConflict::try_from)
            .collect::<Result<Vec<ActiveVesselConflict>, PostgresError>>()
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
        self.all_ais_impl()
            .await
            .unwrap()
            .into_iter()
            .map(AisPosition::try_from)
            .collect::<Result<Vec<AisPosition>, PostgresError>>()
            .unwrap()
    }
    async fn all_vms(&self) -> Vec<VmsPosition> {
        self.all_vms_impl()
            .await
            .unwrap()
            .into_iter()
            .map(VmsPosition::try_from)
            .collect::<Result<Vec<VmsPosition>, PostgresError>>()
            .unwrap()
    }
    async fn all_ais_vms(&self) -> Vec<AisVmsPosition> {
        self.all_ais_vms_impl()
            .await
            .unwrap()
            .into_iter()
            .map(AisVmsPosition::try_from)
            .collect::<Result<Vec<AisVmsPosition>, PostgresError>>()
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
            .collect::<Result<Vec<PortDockPoint>, _>>()
            .unwrap()
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
    ) -> Result<(), InsertError> {
        self.add_deprecated_delivery_point_impl(old, new)
            .await
            .change_context(InsertError)
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
impl AisMigratorDestination for PostgresAdapter {
    async fn add_mmsis(&self, mmsis: Vec<Mmsi>) -> Result<(), InsertError> {
        self.add_mmsis_impl(mmsis).await.change_context(InsertError)
    }
    async fn migrate_ais_data(
        &self,
        mmsi: Mmsi,
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
impl WebApiOutboundPort for PostgresAdapter {
    fn fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        week: u32,
        limit: u32,
    ) -> PinBoxStream<'_, FishingWeightPrediction, QueryError> {
        convert_stream(self.fishing_weight_predictions_impl(model_id, species, week, limit)).boxed()
    }
    fn all_fishing_weight_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingWeightPrediction, QueryError> {
        convert_stream(self.all_fishing_weight_predictions_impl(model_id)).boxed()
    }

    fn all_fishing_spot_predictions(&self) -> PinBoxStream<'_, FishingSpotPrediction, QueryError> {
        self.all_fishing_spot_predictions_impl()
            .map_err(|e| e.change_context(QueryError))
            .boxed()
    }
    async fn fishing_spot_prediction(
        &self,
        species: SpeciesGroup,
        week: u32,
    ) -> Result<Option<FishingSpotPrediction>, QueryError> {
        self.fishing_spot_prediction_impl(species, week)
            .await
            .change_context(QueryError)
    }
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        permission: AisPermission,
    ) -> PinBoxStream<'_, AisPosition, QueryError> {
        convert_stream(self.ais_positions_impl(mmsi, range, permission)).boxed()
    }
    fn vms_positions(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> PinBoxStream<'_, VmsPosition, QueryError> {
        convert_stream(self.vms_positions_impl(call_sign, range)).boxed()
    }

    fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
        permission: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition, QueryError> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, permission)).boxed()
    }

    fn species(&self) -> PinBoxStream<'_, Species, QueryError> {
        convert_stream(self.species_impl()).boxed()
    }

    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir, QueryError> {
        convert_stream(self.species_fiskeridir_impl()).boxed()
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

    async fn detailed_trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError> {
        convert_optional(
            self.detailed_trip_of_haul_impl(haul_id, read_fishing_facility)
                .await
                .change_context(QueryError)?,
        )
    }

    async fn detailed_trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError> {
        convert_optional(
            self.detailed_trip_of_landing_impl(landing_id, read_fishing_facility)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn detailed_trip_of_partial_landing(
        &self,
        landing_id: String,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError> {
        convert_optional(
            self.detailed_trip_of_partial_landing_impl(landing_id, read_fishing_facility)
                .await
                .change_context(QueryError)?,
        )
    }

    fn detailed_trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<PinBoxStream<'_, TripDetailed, QueryError>, QueryError> {
        let stream = self
            .detailed_trips_impl(query, read_fishing_facility)
            .change_context(QueryError)?;
        Ok(convert_stream(stream).boxed())
    }

    async fn current_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> Result<Option<CurrentTrip>, QueryError> {
        convert_optional(
            self.current_trip_impl(vessel_id, read_fishing_facility)
                .await
                .change_context(QueryError)?,
        )
    }

    fn landings(
        &self,
        query: LandingsQuery,
    ) -> Result<PinBoxStream<'_, Landing, QueryError>, QueryError> {
        let stream = self.landings_impl(query).change_context(QueryError)?;
        Ok(convert_stream(stream).boxed())
    }

    async fn landing_matrix(
        &self,
        query: &LandingMatrixQuery,
    ) -> Result<LandingMatrix, QueryError> {
        let active_filter = query.active_filter;
        let args = LandingMatrixArgs::try_from(query.clone()).change_context(QueryError)?;

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
            dates: dates
                .change_context(QueryError)?
                .change_context(QueryError)?,
            length_group: length_group
                .change_context(QueryError)?
                .change_context(QueryError)?,
            gear_group: gear_group
                .change_context(QueryError)?
                .change_context(QueryError)?,
            species_group: species_group
                .change_context(QueryError)?
                .change_context(QueryError)?,
        })
    }

    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> Result<HaulsMatrix, QueryError> {
        let active_filter = query.active_filter;
        let args = HaulsMatrixArgs::try_from(query.clone()).change_context(QueryError)?;

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
            dates: dates
                .change_context(QueryError)?
                .change_context(QueryError)?,
            length_group: length_group
                .change_context(QueryError)?
                .change_context(QueryError)?,
            gear_group: gear_group
                .change_context(QueryError)?
                .change_context(QueryError)?,
            species_group: species_group
                .change_context(QueryError)?
                .change_context(QueryError)?,
        })
    }

    fn fishing_facilities(
        &self,
        query: FishingFacilitiesQuery,
    ) -> PinBoxStream<'_, FishingFacility, QueryError> {
        convert_stream(self.fishing_facilities_impl(query)).boxed()
    }

    async fn get_user(&self, user_id: BarentswatchUserId) -> Result<Option<User>, QueryError> {
        convert_optional(
            self.get_user_impl(user_id)
                .await
                .change_context(QueryError)?,
        )
    }

    fn delivery_points(&self) -> PinBoxStream<'_, DeliveryPoint, QueryError> {
        convert_stream(self.delivery_points_impl()).boxed()
    }

    fn weather(
        &self,
        query: WeatherQuery,
    ) -> Result<PinBoxStream<'_, Weather, QueryError>, QueryError> {
        let stream = self.weather_impl(query).change_context(QueryError)?;
        Ok(convert_stream(stream).boxed())
    }

    fn weather_locations(&self) -> PinBoxStream<'_, WeatherLocation, QueryError> {
        convert_stream(self.weather_locations_impl()).boxed()
    }
}

#[async_trait]
impl WebApiInboundPort for PostgresAdapter {
    async fn update_user(&self, user: User) -> Result<(), UpdateError> {
        self.update_user_impl(user)
            .await
            .change_context(UpdateError)
    }
}

#[async_trait]
impl ScraperInboundPort for PostgresAdapter {
    async fn add_fishing_facilities(
        &self,
        facilities: Vec<FishingFacility>,
    ) -> Result<(), InsertError> {
        self.add_fishing_facilities_impl(facilities)
            .await
            .change_context(InsertError)
    }
    async fn add_register_vessels(
        &self,
        vessels: Vec<fiskeridir_rs::RegisterVessel>,
    ) -> Result<(), InsertError> {
        self.add_register_vessels_full(vessels)
            .await
            .change_context(InsertError)
    }
    async fn add_landings(
        &self,
        landings: Box<
            dyn Iterator<Item = Result<fiskeridir_rs::Landing, fiskeridir_rs::Error>> + Send + Sync,
        >,
        data_year: u32,
    ) -> Result<(), InsertError> {
        self.add_landings_impl(landings, data_year)
            .await
            .change_context(InsertError)
    }
    async fn add_ers_dca(
        &self,
        ers_dca: Box<
            dyn Iterator<Item = Result<fiskeridir_rs::ErsDca, fiskeridir_rs::Error>> + Send + Sync,
        >,
    ) -> Result<(), InsertError> {
        self.add_ers_dca_impl(ers_dca)
            .await
            .change_context(InsertError)
    }
    async fn add_ers_dep(&self, ers_dep: Vec<fiskeridir_rs::ErsDep>) -> Result<(), InsertError> {
        let set = ErsDepSet::new(ers_dep).change_context(InsertError)?;
        self.add_ers_dep_set(set).await.change_context(InsertError)
    }
    async fn add_ers_por(&self, ers_por: Vec<fiskeridir_rs::ErsPor>) -> Result<(), InsertError> {
        let set = ErsPorSet::new(ers_por).change_context(InsertError)?;
        self.add_ers_por_set(set).await.change_context(InsertError)
    }
    async fn add_ers_tra(&self, ers_tra: Vec<fiskeridir_rs::ErsTra>) -> Result<(), InsertError> {
        let set = ErsTraSet::new(ers_tra).change_context(InsertError)?;
        self.add_ers_tra_set(set).await.change_context(InsertError)
    }
    async fn add_vms(&self, vms: Vec<fiskeridir_rs::Vms>) -> Result<(), InsertError> {
        self.add_vms_impl(vms).await.change_context(InsertError)
    }
    async fn add_aqua_culture_register(
        &self,
        entries: Vec<fiskeridir_rs::AquaCultureEntry>,
    ) -> Result<(), InsertError> {
        self.add_aqua_culture_register_impl(entries)
            .await
            .change_context(InsertError)
    }
    async fn add_mattilsynet_delivery_points(
        &self,
        delivery_points: Vec<MattilsynetDeliveryPoint>,
    ) -> Result<(), InsertError> {
        self.add_mattilsynet_delivery_points_impl(delivery_points)
            .await
            .change_context(InsertError)
    }
    async fn add_weather(&self, weather: Vec<NewWeather>) -> Result<(), InsertError> {
        self.add_weather_impl(weather)
            .await
            .change_context(InsertError)
    }
    async fn add_ocean_climate(
        &self,
        ocean_climate: Vec<NewOceanClimate>,
    ) -> Result<(), InsertError> {
        self.add_ocean_climate_impl(ocean_climate)
            .await
            .change_context(InsertError)
    }
}

#[async_trait]
impl ScraperOutboundPort for PostgresAdapter {
    async fn latest_fishing_facility_update(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> Result<Option<DateTime<Utc>>, QueryError> {
        self.latest_fishing_facility_update_impl(source)
            .await
            .change_context(QueryError)
    }
    async fn latest_weather_timestamp(&self) -> Result<Option<DateTime<Utc>>, QueryError> {
        self.latest_weather_timestamp_impl()
            .await
            .change_context(QueryError)
    }
    async fn latest_ocean_climate_timestamp(&self) -> Result<Option<DateTime<Utc>>, QueryError> {
        self.latest_ocean_climate_timestamp_impl()
            .await
            .change_context(QueryError)
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
impl ScraperFileHashOutboundPort for PostgresAdapter {
    async fn get_hash(&self, id: &FileHashId) -> Result<Option<String>, QueryError> {
        self.get_hash_impl(id).await.change_context(QueryError)
    }
}

#[async_trait]
impl TripAssemblerOutboundPort for PostgresAdapter {
    async fn ports(&self) -> Result<Vec<Port>, QueryError> {
        convert_vec(self.ports_impl().await.change_context(QueryError)?)
    }

    async fn dock_points(&self) -> Result<Vec<PortDockPoint>, QueryError> {
        convert_vec(self.dock_points_impl().await.change_context(QueryError)?)
    }

    async fn all_vessels(&self) -> Result<Vec<Vessel>, QueryError> {
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }
    async fn trip_calculation_timer(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Option<TripCalculationTimer>, QueryError> {
        Ok(self
            .trip_calculation_timer_impl(vessel_id, trip_assembler_id)
            .await
            .change_context(QueryError)?
            .map(TripCalculationTimer::from))
    }
    async fn trip_prior_to_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: &DateTime<Utc>,
        bound: Bound,
    ) -> Result<Option<Trip>, QueryError> {
        convert_optional(match bound {
            Bound::Inclusive => self
                .trip_prior_to_timestamp_inclusive(vessel_id, timestamp)
                .await
                .change_context(QueryError)?,
            Bound::Exclusive => self
                .trip_prior_to_timestamp_exclusive(vessel_id, timestamp)
                .await
                .change_context(QueryError)?,
        })
    }
    async fn relevant_events(
        &self,
        vessel_id: FiskeridirVesselId,
        period: &QueryRange,
        event_types: RelevantEventType,
    ) -> Result<Vec<VesselEventDetailed>, QueryError> {
        convert_vec(match event_types {
            RelevantEventType::Landing => self
                .landing_events(vessel_id, period)
                .await
                .change_context(QueryError),
            RelevantEventType::ErsPorAndDep => self
                .ers_por_and_dep_events(vessel_id, period)
                .await
                .change_context(QueryError),
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
    ) -> Result<Vec<AisVmsPosition>, QueryError> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All))
            .try_collect()
            .await
    }
    async fn delivery_points_associated_with_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_landing_coverage: &DateRange,
    ) -> Result<Vec<DeliveryPoint>, QueryError> {
        convert_vec(
            self.delivery_points_associated_with_trip_impl(vessel_id, trip_landing_coverage)
                .await
                .change_context(QueryError)?,
        )
    }
}

#[async_trait]
impl VesselBenchmarkOutbound for PostgresAdapter {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError> {
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }
    async fn sum_trip_time(&self, id: FiskeridirVesselId) -> Result<Option<Duration>, QueryError> {
        self.sum_trip_time_impl(id).await.change_context(QueryError)
    }
    async fn sum_landing_weight(&self, id: FiskeridirVesselId) -> Result<Option<f64>, QueryError> {
        self.sum_landing_weight_impl(id)
            .await
            .change_context(QueryError)
    }
}

#[async_trait]
impl VesselBenchmarkInbound for PostgresAdapter {
    async fn add_output(&self, values: Vec<VesselBenchmarkOutput>) -> Result<(), InsertError> {
        self.add_benchmark_outputs(values)
            .await
            .change_context(InsertError)
    }
}

#[async_trait]
impl HaulDistributorInbound for PostgresAdapter {
    async fn add_output(&self, values: Vec<HaulDistributionOutput>) -> Result<(), UpdateError> {
        self.add_haul_distribution_output(values)
            .await
            .change_context(UpdateError)
    }
}

#[async_trait]
impl HaulDistributorOutbound for PostgresAdapter {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError> {
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }

    async fn catch_locations(&self) -> Result<Vec<CatchLocation>, QueryError> {
        convert_vec(
            self.catch_locations_impl(WeatherLocationOverlap::All)
                .await
                .change_context(QueryError)?,
        )
    }

    async fn haul_messages_of_vessel(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<HaulMessage>, QueryError> {
        convert_vec(
            self.haul_messages_of_vessel_impl(vessel_id)
                .await
                .change_context(QueryError)?,
        )
    }

    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<AisVmsPosition>, QueryError> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All))
            .try_collect()
            .await
    }
}

#[async_trait]
impl TripPipelineOutbound for PostgresAdapter {
    async fn trips_without_distance(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>, QueryError> {
        convert_vec(
            self.trips_without_distance_impl(vessel_id)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn trips_without_precision(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<Trip>, QueryError> {
        convert_vec(
            self.trips_without_precision_impl(vessel_id)
                .await
                .change_context(QueryError)?,
        )
    }
}

#[async_trait]
impl TripPipelineInbound for PostgresAdapter {
    async fn update_preferred_trip_assemblers(&self) -> Result<(), UpdateError> {
        self.update_preferred_trip_assemblers_impl()
            .await
            .change_context(UpdateError)
    }
    async fn update_trip(&self, update: TripUpdate) -> Result<(), UpdateError> {
        self.update_trip_impl(update)
            .await
            .change_context(UpdateError)
    }
    async fn add_trip_set(&self, value: TripSet) -> Result<(), InsertError> {
        self.add_trip_set_impl(value)
            .await
            .change_context(InsertError)
    }
    async fn refresh_detailed_trips(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<(), UpdateError> {
        self.refresh_detailed_trips_impl(vessel_id)
            .await
            .change_context(UpdateError)
    }
}

#[async_trait]
impl MatrixCacheVersion for PostgresAdapter {
    async fn increment(&self) -> Result<(), UpdateError> {
        self.increment_duckdb_version()
            .await
            .change_context(UpdateError)
    }
}

#[async_trait]
impl VerificationOutbound for PostgresAdapter {
    async fn verify_database(&self) -> Result<(), QueryError> {
        self.verify_database_impl().await.change_context(QueryError)
    }
}

#[async_trait]
impl MLModelsOutbound for PostgresAdapter {
    async fn model(&self, model_id: ModelId) -> Result<Vec<u8>, QueryError> {
        self.model_impl(model_id).await.change_context(QueryError)
    }
    async fn fishing_weight_predictor_training_data(
        &self,
        model_id: ModelId,
        weather_data: WeatherData,
        limit: Option<u32>,
    ) -> Result<Vec<WeightPredictorTrainingData>, QueryError> {
        Ok(self
            .fishing_weight_predictor_training_data_impl(model_id, weather_data, limit)
            .await
            .change_context(QueryError)?
            .into_iter()
            .map(WeightPredictorTrainingData::from)
            .collect())
    }

    async fn fishing_spot_predictor_training_data(
        &self,
    ) -> Result<Vec<FishingSpotTrainingData>, QueryError> {
        convert_vec(
            self.fishing_spot_predictor_training_data_impl()
                .await
                .change_context(QueryError)?,
        )
    }

    async fn commit_hauls_training(
        &self,
        model_id: ModelId,
        hauls: Vec<TrainingHaul>,
    ) -> Result<(), InsertError> {
        self.commit_hauls_training_impl(model_id, hauls)
            .await
            .change_context(InsertError)
    }
}

#[async_trait]
impl MLModelsInbound for PostgresAdapter {
    async fn catch_location_weather(
        &self,
        year: u32,
        week: u32,
        catch_location_id: &CatchLocationId,
    ) -> Result<Option<CatchLocationWeather>, QueryError> {
        self.catch_location_weather_impl(year, week, catch_location_id)
            .await
            .change_context(QueryError)
    }
    async fn species_caught_with_traal(&self) -> Result<Vec<SpeciesGroup>, QueryError> {
        self.species_caught_with_traal_impl()
            .await
            .change_context(QueryError)
    }
    async fn existing_fishing_weight_predictions(
        &self,
        model_id: ModelId,
        year: u32,
    ) -> Result<Vec<FishingWeightPrediction>, QueryError> {
        convert_vec(
            self.existing_fishing_weight_predictions_impl(model_id, year)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn existing_fishing_spot_predictions(
        &self,
        year: u32,
    ) -> Result<Vec<FishingSpotPrediction>, QueryError> {
        self.existing_fishing_spot_predictions_impl(year)
            .await
            .change_context(QueryError)
    }
    async fn catch_locations(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> Result<Vec<CatchLocation>, QueryError> {
        convert_vec(
            self.catch_locations_impl(overlap)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn add_fishing_spot_predictions(
        &self,
        predictions: Vec<NewFishingSpotPrediction>,
    ) -> Result<(), InsertError> {
        self.add_fishing_spot_predictions_impl(predictions)
            .await
            .change_context(InsertError)
    }
    async fn add_fishing_weight_predictions(
        &self,
        predictions: Vec<NewFishingWeightPrediction>,
    ) -> Result<(), InsertError> {
        self.add_weight_predictions_impl(predictions)
            .await
            .change_context(InsertError)
    }
    async fn save_model(&self, model_id: ModelId, model: &[u8]) -> Result<(), InsertError> {
        self.save_model_impl(model_id, model)
            .await
            .change_context(InsertError)
    }
}

#[async_trait]
impl HaulWeatherInbound for PostgresAdapter {
    async fn add_haul_weather(&self, values: Vec<HaulWeatherOutput>) -> Result<(), UpdateError> {
        self.add_haul_weather_impl(values)
            .await
            .change_context(UpdateError)
    }
}

#[async_trait]
impl HaulWeatherOutbound for PostgresAdapter {
    async fn all_vessels(&self) -> Result<Vec<Vessel>, QueryError> {
        convert_stream(self.fiskeridir_ais_vessel_combinations())
            .try_collect()
            .await
    }
    async fn haul_messages_of_vessel_without_weather(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<HaulMessage>, QueryError> {
        convert_vec(
            self.haul_messages_of_vessel_without_weather_impl(vessel_id)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<AisVmsPosition>, QueryError> {
        convert_stream(self.ais_vms_positions_impl(mmsi, call_sign, range, AisPermission::All))
            .try_collect()
            .await
    }
    async fn weather_locations(&self) -> Result<Vec<WeatherLocation>, QueryError> {
        convert_stream(self.weather_locations_impl())
            .try_collect()
            .await
    }
    async fn haul_weather(&self, query: WeatherQuery) -> Result<Option<HaulWeather>, QueryError> {
        convert_optional(
            self.haul_weather_impl(query)
                .await
                .change_context(QueryError)?,
        )
    }
    async fn haul_ocean_climate(
        &self,
        query: OceanClimateQuery,
    ) -> Result<Option<HaulOceanClimate>, QueryError> {
        convert_optional(
            self.haul_ocean_climate_impl(query)
                .await
                .change_context(QueryError)?,
        )
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

pub(crate) fn convert_vec<A, B>(val: Vec<A>) -> Result<Vec<B>, QueryError>
where
    B: std::convert::TryFrom<A, Error = Report<PostgresError>>,
{
    val.into_iter()
        .map(B::try_from)
        .collect::<std::result::Result<Vec<_>, <B as std::convert::TryFrom<A>>::Error>>()
        .change_context(QueryError)
}
