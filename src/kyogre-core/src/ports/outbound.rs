use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;

#[async_trait]
pub trait AisMigratorSource {
    async fn ais_positions(
        &self,
        mmsi: i32,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AisPosition>, QueryError>;
    async fn existing_mmsis(&self) -> Result<Vec<i32>, QueryError>;
}
#[async_trait]
pub trait WebApiPort {
    async fn ais_positions(
        &self,
        mmsi: i32,
        range: &DateRange,
    ) -> Result<Vec<AisPosition>, QueryError>;
    async fn species(&self) -> Result<Vec<Species>, QueryError>;
    async fn species_fiskeridir(&self) -> Result<Vec<SpeciesFiskeridir>, QueryError>;
    async fn species_groups(&self) -> Result<Vec<SpeciesGroup>, QueryError>;
    async fn species_main_groups(&self) -> Result<Vec<SpeciesMainGroup>, QueryError>;
    async fn species_fao(&self) -> Result<Vec<SpeciesFao>, QueryError>;
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError>;
}

#[async_trait]
pub trait TripAssemblerOutboundPort: Send + Sync {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn trip_calculation_timers(&self) -> Result<Vec<TripCalculationTimer>, QueryError>;
    async fn conflicts(
        &self,
        id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, QueryError>;
    async fn landing_dates(
        &self,
        vessel_id: i64,
        start: &DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, QueryError>;
    async fn most_recent_trip(
        &self,
        vessel_id: i64,
        assembler_id: TripAssemblerId,
    ) -> Result<Option<Trip>, QueryError>;
    async fn departure_of_trip(&self, trip_id: i64) -> Result<Departure, QueryError>;
    async fn ers_arrivals(
        &self,
        vessel_id: i64,
        start: &DateTime<Utc>,
        filter: ArrivalFilter,
    ) -> Result<Arrival, QueryError>;
    async fn ers_departures(
        &self,
        vessel_id: i64,
        start: &DateTime<Utc>,
    ) -> Result<Departure, QueryError>;
    async fn trip_prior_to(
        &self,
        vessel_id: i64,
        assembler_id: TripAssemblerId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, QueryError>;
}

#[async_trait]
pub trait TripPrecisionOutboundPort: Send + Sync {
    async fn ports_of_trip(&self, trip_id: i64) -> Result<TripPorts, QueryError>;
    async fn dock_points_of_trip(&self, trip_id: i64) -> Result<TripDockPoints, QueryError>;
    async fn ais_positions(
        &self,
        vessel_id: i64,
        range: &DateRange,
    ) -> Result<Vec<AisPosition>, QueryError>;
    async fn trip_prior_to(
        &self,
        vessel_id: i64,
        assembler_id: TripAssemblerId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, QueryError>;
    async fn delivery_points_associated_with_trip(
        &self,
        trip_id: i64,
    ) -> Result<Vec<DeliveryPoint>, QueryError>;

    async fn trips_without_precision(
        &self,
        vessel_id: i64,
        assembler_id: TripAssemblerId,
    ) -> Result<Vec<Trip>, QueryError>;
}
