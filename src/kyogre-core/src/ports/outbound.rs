use crate::{
    AisPosition, Arrival, ArrivalFilter, DateRange, Departure, QueryError, Trip,
    TripAssemblerConflict, TripAssemblerId,
};

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
}

#[async_trait]
pub trait TripAssemblerOutboundPort: Send + Sync {
    async fn conflicts(
        &self,
        id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, QueryError>;
    async fn landing_dates(
        &self,
        vessel_id: i64,
        range: &DateRange,
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
        range: &DateRange,
        filter: ArrivalFilter,
    ) -> Result<Arrival, QueryError>;
    async fn ers_departures(
        &self,
        vessel_id: i64,
        range: &DateRange,
    ) -> Result<Departure, QueryError>;
}
