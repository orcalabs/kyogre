use std::pin::Pin;

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use error_stack::Result;
use fiskeridir_rs::{CallSign, DeliveryPointId, LandingId};
use futures::Stream;

#[async_trait]
pub trait AisMigratorSource {
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AisPosition>, QueryError>;
    async fn existing_mmsis(&self) -> Result<Vec<Mmsi>, QueryError>;
}

pub type PinBoxStream<'a, T, E> = Pin<Box<dyn Stream<Item = Result<T, E>> + Send + 'a>>;

#[async_trait]
pub trait WebApiOutboundPort {
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisPosition, QueryError>;
    fn vms_positions(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> PinBoxStream<'_, VmsPosition, QueryError>;
    fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
        user_policy: AisPermission,
    ) -> PinBoxStream<'_, AisVmsPosition, QueryError>;
    fn species(&self) -> PinBoxStream<'_, Species, QueryError>;
    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir, QueryError>;
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao, QueryError>;
    fn vessels(&self) -> Pin<Box<dyn Stream<Item = Result<Vessel, QueryError>> + Send + '_>>;
    fn hauls(&self, query: HaulsQuery) -> Result<PinBoxStream<'_, Haul, QueryError>, QueryError>;
    fn detailed_trips_of_vessel(
        &self,
        id: FiskeridirVesselId,
        pagination: Pagination<Trips>,
        ordering: Ordering,
        read_fishing_facility: bool,
    ) -> Result<PinBoxStream<'_, TripDetailed, QueryError>, QueryError>;
    fn detailed_trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<PinBoxStream<'_, TripDetailed, QueryError>, QueryError>;
    async fn detailed_trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError>;
    async fn detailed_trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError>;
    async fn current_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        read_fishing_facility: bool,
    ) -> Result<Option<CurrentTrip>, QueryError>;
    async fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> Result<HaulsMatrix, QueryError>;
    fn landings(
        &self,
        query: LandingsQuery,
    ) -> Result<PinBoxStream<'_, Landing, QueryError>, QueryError>;
    async fn landing_matrix(&self, query: &LandingMatrixQuery)
        -> Result<LandingMatrix, QueryError>;
    fn fishing_facilities(
        &self,
        query: FishingFacilitiesQuery,
    ) -> PinBoxStream<'_, FishingFacility, QueryError>;
    async fn get_user(&self, user_id: BarentswatchUserId) -> Result<Option<User>, QueryError>;
    fn delivery_points(&self) -> PinBoxStream<'_, DeliveryPoint, QueryError>;
    fn weather(
        &self,
        query: WeatherQuery,
    ) -> Result<PinBoxStream<'_, Weather, QueryError>, QueryError>;
    fn benchmark(&self) -> PinBoxStream<'_,Benchmark,QueryError>;
}

#[async_trait]
pub trait TripAssemblerOutboundPort: Send + Sync {
    async fn all_vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn trip_calculation_timers(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripCalculationTimer>, QueryError>;
    async fn conflicts(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, QueryError>;
    async fn trip_prior_to_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: &DateTime<Utc>,
        bound: Bound,
    ) -> Result<Option<Trip>, QueryError>;
    async fn relevant_events(
        &self,
        vessel_id: FiskeridirVesselId,
        period: &QueryRange,
        event_types: RelevantEventType,
    ) -> Result<Vec<VesselEventDetailed>, QueryError>;
    async fn add_trips(
        &self,
        vessel_id: FiskeridirVesselId,
        new_trip_calculation_time: DateTime<Utc>,
        conflict_strategy: TripsConflictStrategy,
        trips: Vec<NewTrip>,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<(), InsertError>;
}

#[async_trait]
pub trait TripPrecisionOutboundPort: Send + Sync {
    async fn all_vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn ports_of_trip(&self, trip_id: TripId) -> Result<TripPorts, QueryError>;
    async fn dock_points_of_trip(&self, trip_id: TripId) -> Result<TripDockPoints, QueryError>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<AisVmsPosition>, QueryError>;
    async fn trip_prior_to_timestamp(
        &self,
        vessel_id: FiskeridirVesselId,
        timestamp: &DateTime<Utc>,
        bound: Bound,
    ) -> Result<Option<Trip>, QueryError>;
    async fn delivery_points_associated_with_trip(
        &self,
        trip_id: TripId,
    ) -> Result<Vec<DeliveryPoint>, QueryError>;

    async fn trips_without_precision(
        &self,
        vessel_id: FiskeridirVesselId,
        assembler_id: TripAssemblerId,
    ) -> Result<Vec<Trip>, QueryError>;
}

#[async_trait]
pub trait VesselBenchmarkOutbound: Send + Sync {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn sum_trip_time(&self, id: FiskeridirVesselId) -> Result<Option<Duration>, QueryError>;
    async fn sum_trip_distance(&self, id: FiskeridirVesselId) -> Result<Option<f64>, QueryError>;
    async fn sum_landing_weight(&self, id: FiskeridirVesselId) -> Result<Option<f64>, QueryError>;
}

#[async_trait]
pub trait MatrixCacheOutbound: Send + Sync {
    async fn hauls_matrix(
        &self,
        query: HaulsMatrixQuery,
    ) -> Result<Option<HaulsMatrix>, QueryError>;
    async fn landing_matrix(
        &self,
        query: LandingMatrixQuery,
    ) -> Result<Option<LandingMatrix>, QueryError>;
}

#[async_trait]
pub trait HaulDistributorOutbound: Send + Sync {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn catch_locations(&self) -> Result<Vec<CatchLocation>, QueryError>;
    async fn haul_messages_of_vessel(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<HaulMessage>, QueryError>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<AisVmsPosition>, QueryError>;
}

#[async_trait]
pub trait TripDistancerOutbound: Send + Sync {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn trips_of_vessel(&self, vessel_id: FiskeridirVesselId)
        -> Result<Vec<Trip>, QueryError>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<AisVmsPosition>, QueryError>;
}

#[async_trait]
pub trait MatrixCacheVersion: Send + Sync {
    async fn increment(&self) -> Result<(), UpdateError>;
}

#[async_trait]
pub trait VerificationOutbound: Send + Sync {
    async fn verify_database(&self) -> Result<(), QueryError>;
}

#[async_trait]
pub trait HaulWeatherOutbound: Send + Sync {
    async fn all_vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn haul_messages_of_vessel_without_weather(
        &self,
        vessel_id: FiskeridirVesselId,
    ) -> Result<Vec<HaulMessage>, QueryError>;
    async fn ais_vms_positions(
        &self,
        mmsi: Option<Mmsi>,
        call_sign: Option<&CallSign>,
        range: &DateRange,
    ) -> Result<Vec<AisVmsPosition>, QueryError>;
    async fn weather_locations(&self) -> Result<Vec<WeatherLocation>, QueryError>;
    async fn haul_weather(&self, query: WeatherQuery) -> Result<Option<HaulWeather>, QueryError>;
    async fn haul_ocean_climate(
        &self,
        query: OceanClimateQuery,
    ) -> Result<Option<HaulOceanClimate>, QueryError>;
}

#[async_trait]
pub trait ScraperFileHashOutboundPort {
    async fn get_hash(&self, id: &FileHashId) -> Result<Option<String>, QueryError>;
}

#[async_trait]
pub trait TestHelperOutbound: Send + Sync {
    async fn all_dep(&self) -> Vec<Departure>;
    async fn all_por(&self) -> Vec<Arrival>;
    async fn all_ais(&self) -> Vec<AisPosition>;
    async fn all_vms(&self) -> Vec<VmsPosition>;
    async fn all_ais_vms(&self) -> Vec<AisVmsPosition>;
    async fn delivery_points_log(&self) -> Vec<serde_json::Value>;
    async fn port(&self, port_id: &str) -> Option<Port>;
    async fn delivery_point(&self, id: &DeliveryPointId) -> Option<DeliveryPoint>;
    async fn dock_points_of_port(&self, port_id: &str) -> Vec<PortDockPoint>;
}
