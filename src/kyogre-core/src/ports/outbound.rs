use std::pin::Pin;

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;
use futures::Stream;

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
    fn ais_positions(
        &self,
        mmsi: i32,
        range: &DateRange,
    ) -> Pin<Box<dyn Stream<Item = Result<AisPosition, QueryError>> + '_>>;
    fn species(&self) -> Pin<Box<dyn Stream<Item = Result<Species, QueryError>> + '_>>;
    fn species_fiskeridir(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<SpeciesFiskeridir, QueryError>> + '_>>;
    fn species_groups(&self) -> Pin<Box<dyn Stream<Item = Result<SpeciesGroup, QueryError>> + '_>>;
    fn species_main_groups(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<SpeciesMainGroup, QueryError>> + '_>>;
    fn species_fao(&self) -> Pin<Box<dyn Stream<Item = Result<SpeciesFao, QueryError>> + '_>>;
    fn vessels(&self) -> Pin<Box<dyn Stream<Item = Result<Vessel, QueryError>> + Send + '_>>;
    fn hauls(
        &self,
        query: HaulsQuery,
    ) -> Pin<Box<dyn Stream<Item = Result<Haul, QueryError>> + '_>>;
    async fn hauls_grid(&self, query: HaulsQuery) -> Result<HaulsGrid, QueryError>;
}

#[async_trait]
pub trait TripAssemblerOutboundPort: Send + Sync {
    async fn vessels(&self) -> Result<Vec<Vessel>, QueryError>;
    async fn trip_calculation_timers(
        &self,
        trip_assembler_id: TripAssemblerId,
    ) -> Result<Vec<TripCalculationTimer>, QueryError>;
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
    async fn trip_at_or_prior_to(
        &self,
        fiskeridir_vessel_id: i64,
        trip_assembler_id: TripAssemblerId,
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
