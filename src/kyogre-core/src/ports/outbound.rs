use std::pin::Pin;

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::{CallSign, DataFileId, DeliveryPointId, LandingId, SpeciesGroup};
use futures::Stream;

#[async_trait]
pub trait AisMigratorSource {
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> CoreResult<Vec<AisPosition>>;
    async fn existing_mmsis(&self) -> CoreResult<Vec<Mmsi>>;
}

pub type PinBoxStream<'a, T> = Pin<Box<dyn Stream<Item = CoreResult<T>> + Send + 'a>>;

#[async_trait]
pub trait WebApiOutboundPort {
    fn ais_current_positions(
        &self,
        limit: Option<DateTime<Utc>>,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisPosition>;
    fn ais_vms_area_positions(
        &self,
        x1: f64,
        x2: f64,
        y1: f64,
        y2: f64,
        date_limit: NaiveDate,
    ) -> PinBoxStream<'_, AisVmsAreaCount>;
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisPosition>;
    fn vms_positions(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> PinBoxStream<'_, VmsPosition>;
    fn ais_vms_positions(
        &self,
        params: AisVmsParams,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition>;
    fn species(&self) -> PinBoxStream<'_, Species>;
    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir>;
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao>;
    fn vessels(&self) -> Pin<Box<dyn Stream<Item = CoreResult<Vessel>> + Send + '_>>;
    fn hauls(&self, query: HaulsQuery) -> PinBoxStream<'_, Haul>;
    async fn vessel_benchmarks(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
    ) -> CoreResult<VesselBenchmarks>;
    fn detailed_trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> PinBoxStream<'_, TripDetailed>;
    async fn detailed_trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>>;
    async fn detailed_trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>>;
    async fn current_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<CurrentTrip>>;
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> CoreResult<HaulsMatrix>;
    fn landings(&self, query: LandingsQuery) -> PinBoxStream<'_, Landing>;
    async fn landing_matrix(&self, query: &LandingMatrixQuery) -> CoreResult<LandingMatrix>;
    fn fishing_facilities(
        &self,
        query: FishingFacilitiesQuery,
    ) -> PinBoxStream<'_, FishingFacility>;
    async fn get_user(&self, user_id: BarentswatchUserId) -> CoreResult<Option<User>>;
    fn delivery_points(&self) -> PinBoxStream<'_, DeliveryPoint>;
    fn weather(&self, query: WeatherQuery) -> PinBoxStream<'_, Weather>;
    fn weather_locations(&self) -> PinBoxStream<'_, WeatherLocation>;
    async fn fishing_spot_prediction(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
    ) -> CoreResult<Option<FishingSpotPrediction>>;
    fn fishing_weight_predictions(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
        limit: u32,
    ) -> PinBoxStream<'_, FishingWeightPrediction>;
    fn all_fishing_spot_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingSpotPrediction>;
    fn all_fishing_weight_predictions(
        &self,
        model_id: ModelId,
    ) -> PinBoxStream<'_, FishingWeightPrediction>;
    fn fuel_measurements(&self, query: FuelMeasurementsQuery) -> PinBoxStream<'_, FuelMeasurement>;
}

#[async_trait]
pub trait TripAssemblerOutboundPort: Send + Sync {
    async fn all_vessels(&self) -> CoreResult<Vec<Vessel>>;
    async fn trip_calculation_timer(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> CoreResult<Option<TripCalculationTimer>>;
    async fn trip_prior_to_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: &DateTime<Utc>,
        bound: Bound,
    ) -> CoreResult<Option<Trip>>;
    async fn relevant_events(
        &self,
        vessel_id: FiskeridirVesselId,
        period: &QueryRange,
        event_types: RelevantEventType,
    ) -> CoreResult<Vec<VesselEventDetailed>>;
    async fn ports(&self) -> CoreResult<Vec<Port>>;
    async fn dock_points(&self) -> CoreResult<Vec<PortDockPoint>>;
}

#[async_trait]
pub trait TripPrecisionOutboundPort: Send + Sync {
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>>;
    async fn delivery_points_associated_with_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_landing_coverage: &DateRange,
    ) -> CoreResult<Vec<DeliveryPoint>>;
}

#[async_trait]
pub trait VesselBenchmarkOutbound: Send + Sync {
    async fn vessels(&self) -> CoreResult<Vec<Vessel>>;
    async fn sum_trip_time(&self, id: FiskeridirVesselId) -> CoreResult<Option<Duration>>;
    async fn sum_landing_weight(&self, id: FiskeridirVesselId) -> CoreResult<Option<f64>>;
}

#[async_trait]
pub trait MatrixCacheOutbound: Send + Sync {
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> CoreResult<Option<HaulsMatrix>>;
    async fn landing_matrix(&self, query: &LandingMatrixQuery)
        -> CoreResult<Option<LandingMatrix>>;
}

#[async_trait]
pub trait MeilisearchOutbound: Send + Sync {
    async fn trips(
        &self,
        query: &TripsQuery,
        read_fishing_facility: bool,
    ) -> CoreResult<Vec<TripDetailed>>;
    async fn trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>>;
    async fn trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>>;
    async fn hauls(&self, query: &HaulsQuery) -> CoreResult<Vec<Haul>>;
    async fn landings(&self, query: &LandingsQuery) -> CoreResult<Vec<Landing>>;
}

#[async_trait]
pub trait MeilisearchSource: Send + Sync + Clone {
    async fn trips_by_ids(&self, trip_ids: &[TripId]) -> CoreResult<Vec<TripDetailed>>;
    async fn hauls_by_ids(&self, haul_ids: &[HaulId]) -> CoreResult<Vec<Haul>>;
    async fn landings_by_ids(&self, haul_ids: &[LandingId]) -> CoreResult<Vec<Landing>>;
    async fn all_trip_versions(&self) -> CoreResult<Vec<(TripId, i64)>>;
    async fn all_haul_versions(&self) -> CoreResult<Vec<(HaulId, i64)>>;
    async fn all_landing_versions(&self) -> CoreResult<Vec<(LandingId, i64)>>;
}

#[async_trait]
pub trait HaulDistributorOutbound: Send + Sync {
    async fn vessels(&self) -> CoreResult<Vec<Vessel>>;
    async fn catch_locations(&self) -> CoreResult<Vec<CatchLocation>>;
    async fn haul_messages_of_vessel(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<HaulMessage>>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>>;
}

#[async_trait]
pub trait MatrixCacheVersion: Send + Sync {
    async fn increment(&self) -> CoreResult<()>;
}

#[async_trait]
pub trait VerificationOutbound: Send + Sync {
    async fn verify_database(&self) -> CoreResult<()>;
}

#[async_trait]
pub trait HaulWeatherOutbound: Send + Sync {
    async fn all_vessels(&self) -> CoreResult<Vec<Vessel>>;
    async fn haul_messages_of_vessel_without_weather(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<HaulMessage>>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>>;
    async fn weather_locations(&self) -> CoreResult<Vec<WeatherLocation>>;
    async fn haul_weather(&self, query: WeatherQuery) -> CoreResult<Option<HaulWeather>>;
    async fn haul_ocean_climate(
        &self,
        query: OceanClimateQuery,
    ) -> CoreResult<Option<HaulOceanClimate>>;
}

#[async_trait]
pub trait ScraperFileHashOutboundPort {
    async fn get_hashes(&self, ids: &[DataFileId]) -> CoreResult<Vec<(DataFileId, String)>>;
}

#[async_trait]
pub trait TripPipelineOutbound: Send + Sync {
    async fn trips_without_position_layers(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> CoreResult<Vec<Trip>>;
    async fn trips_without_distance(&self, vessel_id: FiskeridirVesselId) -> CoreResult<Vec<Trip>>;
    async fn trips_without_precision(&self, vessel_id: FiskeridirVesselId)
        -> CoreResult<Vec<Trip>>;
}

#[async_trait]
pub trait MLModelsOutbound: Send + Sync {
    async fn save_model(
        &self,
        model_id: ModelId,
        model: &[u8],
        species: SpeciesGroup,
    ) -> CoreResult<()>;
    async fn fishing_spot_predictor_training_data(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        limit: Option<u32>,
    ) -> CoreResult<Vec<FishingSpotTrainingData>>;
    async fn fishing_weight_predictor_training_data(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        weather: WeatherData,
        limit: Option<u32>,
        bycatch_percentage: Option<f64>,
        majority_species_group: bool,
    ) -> CoreResult<Vec<WeightPredictorTrainingData>>;
    async fn commit_hauls_training(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        haul: Vec<TrainingHaul>,
    ) -> CoreResult<()>;
    async fn model(&self, model_id: ModelId, species: SpeciesGroup) -> CoreResult<Vec<u8>>;
    async fn catch_locations_weather_dates(
        &self,
        dates: Vec<NaiveDate>,
    ) -> CoreResult<Vec<CatchLocationWeather>>;
    async fn catch_locations(
        &self,
        overlap: WeatherLocationOverlap,
    ) -> CoreResult<Vec<CatchLocation>>;
    async fn catch_locations_weather(
        &self,
        keys: Vec<(CatchLocationId, NaiveDate)>,
    ) -> CoreResult<Vec<CatchLocationWeather>>;
}

#[async_trait]
pub trait TestHelperOutbound: Send + Sync {
    async fn all_dep(&self) -> Vec<Departure>;
    async fn all_por(&self) -> Vec<Arrival>;
    async fn all_ais(&self) -> Vec<AisPosition>;
    async fn all_vms(&self) -> Vec<VmsPosition>;
    async fn all_ais_vms(&self) -> Vec<AisVmsPosition>;
    async fn active_vessel_conflicts(&self) -> Vec<ActiveVesselConflict>;
    async fn delivery_points_log(&self) -> Vec<serde_json::Value>;
    async fn port(&self, port_id: &str) -> Option<Port>;
    async fn delivery_point(&self, id: &DeliveryPointId) -> Option<DeliveryPoint>;
    async fn dock_points_of_port(&self, port_id: &str) -> Vec<PortDockPoint>;
    async fn trip_assembler_log(&self) -> Vec<TripAssemblerLogEntry>;
}
