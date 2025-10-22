use core::f64;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use fiskeridir_rs::{CallSign, DataFileId, LandingId, OrgId, SpeciesGroup};
use futures::Stream;

use crate::*;

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

pub type PinBoxStream<'a, T> = Pin<Box<dyn Stream<Item = WebApiResult<T>> + Send + 'a>>;

#[async_trait]
pub trait WebApiOutboundPort {
    fn current_positions(
        &self,
        limit: Option<DateTime<Utc>>,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, CurrentPosition>;
    fn current_trip_positions(
        &self,
        vessel_id: FiskeridirVesselId,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition>;
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisPosition>;
    fn vms_positions<'a>(
        &'a self,
        call_sign: &'a CallSign,
        range: &'a DateRange,
    ) -> PinBoxStream<'a, VmsPosition>;
    fn ais_vms_positions(
        &self,
        params: AisVmsParams,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition>;
    fn species(&self) -> PinBoxStream<'_, Species>;
    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir>;
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao>;
    fn vessels(&self) -> PinBoxStream<'_, Vessel>;
    fn hauls(&self, query: HaulsQuery) -> PinBoxStream<'_, Haul>;
    async fn org_benchmarks(
        &self,
        query: &OrgBenchmarkQuery,
    ) -> WebApiResult<Option<OrgBenchmarks>>;
    async fn vessel_benchmarks(
        &self,
        user_id: &BarentswatchUserId,
        call_sign: &CallSign,
    ) -> WebApiResult<VesselBenchmarks>;
    async fn trip_benchmarks(
        &self,
        query: TripBenchmarksQuery,
    ) -> WebApiResult<Vec<TripWithBenchmark>>;
    async fn eeoi(&self, query: EeoiQuery) -> WebApiResult<Option<f64>>;
    async fn fui(&self, query: FuiQuery) -> WebApiResult<Option<f64>>;
    fn detailed_trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> PinBoxStream<'_, TripDetailed>;
    async fn current_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> WebApiResult<Option<CurrentTrip>>;
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> WebApiResult<HaulsMatrix>;
    fn landings(&self, query: LandingsQuery) -> PinBoxStream<'_, Landing>;
    async fn landing_matrix(&self, query: &LandingMatrixQuery) -> WebApiResult<LandingMatrix>;
    async fn average_trip_benchmarks(
        &self,
        query: AverageTripBenchmarksQuery,
    ) -> WebApiResult<AverageTripBenchmarks>;
    async fn average_eeoi(&self, query: AverageEeoiQuery) -> WebApiResult<Option<f64>>;
    async fn average_fui(&self, query: AverageFuiQuery) -> WebApiResult<Option<f64>>;
    async fn live_fuel(&self, query: &LiveFuelQuery) -> WebApiResult<LiveFuel>;
    async fn fuel_estimation(&self, query: &FuelQuery) -> WebApiResult<f64>;
    async fn fuel_estimation_by_org(
        &self,
        query: &FuelQuery,
        org_id: OrgId,
    ) -> WebApiResult<Option<Vec<FuelEntry>>>;
    fn fishing_facilities(
        &self,
        query: FishingFacilitiesQuery,
    ) -> PinBoxStream<'_, FishingFacility>;
    async fn get_user(&self, user_id: BarentswatchUserId) -> WebApiResult<Option<User>>;
    fn delivery_points(&self) -> PinBoxStream<'_, DeliveryPoint>;
    fn weather(&self, query: WeatherQuery) -> PinBoxStream<'_, Weather>;
    fn weather_locations(&self) -> PinBoxStream<'_, WeatherLocation>;
    async fn fishing_spot_prediction(
        &self,
        model_id: ModelId,
        species: SpeciesGroup,
        date: NaiveDate,
    ) -> WebApiResult<Option<FishingSpotPrediction>>;
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
    async fn update_vessel(
        &self,
        call_sign: &CallSign,
        update: &UpdateVessel,
    ) -> WebApiResult<Option<Vessel>>;
}

#[async_trait]
pub trait TripAssemblerOutboundPort: Send + Sync {
    async fn all_vessels(&self) -> CoreResult<Vec<Vessel>>;
    async fn trip_calculation_timer(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
    ) -> CoreResult<Option<TripCalculationTimer>>;
    async fn all_vessel_events(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler: TripAssemblerId,
    ) -> CoreResult<Vec<VesselEventDetailed>>;
    async fn trip_prior_to_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        search_timestamp: TripSearchTimestamp,
        trip_assembler: TripAssemblerId,
    ) -> CoreResult<Option<TripAndSucceedingEvents>>;
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
    async fn ais_vms_positions_with_inside_haul(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>>;
    async fn trip_positions_with_inside_haul(
        &self,
        trip_id: TripId,
    ) -> CoreResult<Vec<AisVmsPosition>>;
    async fn delivery_points_associated_with_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_landing_coverage: &DateRange,
    ) -> CoreResult<Vec<DeliveryPoint>>;

    async fn departure_weights_from_range(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> CoreResult<Vec<DepartureWeight>>;
    async fn haul_weights_from_range(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> CoreResult<Vec<HaulWeight>>;
    async fn vessel_max_cargo_weight(&self, vessel_id: FiskeridirVesselId) -> CoreResult<f64>;
}

#[async_trait]
pub trait TripBenchmarkOutbound: Send + Sync {
    async fn vessels(&self) -> CoreResult<Vec<Vessel>>;
    async fn trip_positions_with_manual(
        &self,
        trip_id: TripId,
    ) -> CoreResult<Vec<TripPositionWithManual>>;
    async fn trips_to_benchmark(&self) -> CoreResult<Vec<BenchmarkTrip>>;
    async fn overlapping_measurment_fuel(
        &self,
        vessel_id: FiskeridirVesselId,
        range: &DateRange,
    ) -> CoreResult<f64>;
}

#[async_trait]
pub trait MatrixCacheOutbound: Send + Sync {
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> CoreResult<HaulsMatrix>;
    async fn landing_matrix(&self, query: &LandingMatrixQuery) -> CoreResult<LandingMatrix>;
}

#[async_trait]
pub trait MeilisearchOutbound: Send + Sync {
    async fn trips(
        &self,
        query: &TripsQuery,
        read_fishing_facility: bool,
    ) -> CoreResult<Vec<TripDetailed>>;
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
pub trait FuelEstimation: Send + Sync {
    // Only used in tests to reduce the amount of estimations generated
    #[cfg(feature = "test")]
    async fn latest_position(&self) -> CoreResult<Option<NaiveDate>>;

    async fn last_run(&self) -> CoreResult<Option<DateTime<Utc>>>;
    async fn add_run(&self) -> CoreResult<()>;

    async fn add_fuel_estimates(&self, estimates: &[NewFuelDayEstimate]) -> CoreResult<()>;
    async fn vessels_with_trips(&self, num_trips: u32) -> CoreResult<Vec<Vessel>>;
    async fn delete_fuel_estimates(&self, vessels: &[FiskeridirVesselId]) -> CoreResult<()>;
    async fn reset_trip_positions_fuel_status(
        &self,
        vessels: &[FiskeridirVesselId],
    ) -> CoreResult<()>;
    async fn dates_to_estimate(
        &self,
        vessel_id: FiskeridirVesselId,
        call_sign: Option<&CallSign>,
        mmsi: Option<Mmsi>,
        end_date: NaiveDate,
    ) -> CoreResult<Vec<NaiveDate>>;
    async fn fuel_estimation_positions(
        &self,
        vessel_id: FiskeridirVesselId,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<DailyFuelEstimationPosition>>;
    async fn vessel_max_cargo_weight(&self, vessel_id: FiskeridirVesselId) -> CoreResult<f64>;
}

#[async_trait]
pub trait CurrentPositionOutbound: Send + Sync {
    async fn vessels(&self) -> CoreResult<Vec<CurrentPositionVessel>>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> CoreResult<Vec<AisVmsPosition>>;
}

#[async_trait]
pub trait VerificationOutbound: Send + Sync {
    /// Runs a set of verification queries to check if certain constraints, which we cannot express
    /// as database constraints, holds.
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
pub trait ScraperOutboundPort {
    async fn latest_fishing_facility_update(
        &self,
        source: Option<FishingFacilityApiSource>,
    ) -> CoreResult<Option<DateTime<Utc>>>;
    async fn latest_weather_timestamp(&self) -> CoreResult<Option<DateTime<Utc>>>;
    async fn latest_ocean_climate_timestamp(&self) -> CoreResult<Option<DateTime<Utc>>>;
    async fn latest_buyer_location_update(&self) -> CoreResult<Option<NaiveDateTime>>;
    async fn latest_weekly_sale(&self) -> CoreResult<Option<NaiveDate>>;
}

#[async_trait]
pub trait ScraperFileHashOutboundPort {
    async fn get_hashes(&self, ids: &[DataFileId]) -> CoreResult<Vec<(DataFileId, String)>>;
}

#[async_trait]
pub trait TripPipelineOutbound: Send + Sync {
    async fn trips_without_position_cargo_weight_distribution(
        &self,
        vessel_id: FiskeridirVesselId,
        limit: u32,
    ) -> CoreResult<Vec<Trip>>;
    async fn trips_without_position_fuel_consumption_distribution(
        &self,
        vessel_id: FiskeridirVesselId,
        limit: u32,
    ) -> CoreResult<Vec<Trip>>;
    async fn trips_without_position_layers(
        &self,
        vessel_id: FiskeridirVesselId,
        limit: u32,
    ) -> CoreResult<Vec<Trip>>;
    async fn trips_without_distance(
        &self,
        vessel_id: FiskeridirVesselId,
        limit: u32,
    ) -> CoreResult<Vec<Trip>>;
    async fn trips_without_precision(
        &self,
        vessel_id: FiskeridirVesselId,
        limit: u32,
    ) -> CoreResult<Vec<Trip>>;
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

#[cfg(feature = "test")]
#[async_trait]
pub trait TestHelperOutbound: Send + Sync {
    async fn all_tra(&self) -> Vec<Tra>;
    async fn all_dep(&self) -> Vec<Departure>;
    async fn all_por(&self) -> Vec<Arrival>;
    async fn all_ais(&self) -> Vec<AisPosition>;
    async fn all_vms(&self) -> Vec<VmsPosition>;
    async fn all_ais_vms(&self) -> Vec<AisVmsPosition>;
    async fn all_fuel_estimates(&self) -> Vec<f64>;
    async fn all_fuel_measurement_ranges(&self) -> Vec<FuelMeasurementRange>;
    async fn sum_fuel_estimates(
        &self,
        start: NaiveDate,
        end: NaiveDate,
        to_skip: &[NaiveDate],
        vessels: Option<&[FiskeridirVesselId]>,
    ) -> f64;
    async fn active_vessel_conflicts(&self) -> Vec<ActiveVesselConflict>;
    async fn delivery_points_log(&self) -> Vec<serde_json::Value>;
    async fn port(&self, port_id: &str) -> Option<Port>;
    async fn delivery_point(&self, id: &fiskeridir_rs::DeliveryPointId) -> Option<DeliveryPoint>;
    async fn dock_points_of_port(&self, port_id: &str) -> Vec<PortDockPoint>;
    async fn trip_assembler_log(&self) -> Vec<TripAssemblerLogEntry>;
    async fn trips_with_benchmark_status(&self, status: ProcessingStatus) -> u32;
    async fn unprocessed_trips(&self) -> u32;
    async fn fuel_estimates_with_status(&self, status: ProcessingStatus) -> u32;
}
