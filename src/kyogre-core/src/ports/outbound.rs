use std::pin::Pin;

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;
use fiskeridir_rs::CallSign;
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

pub type PinBoxStream<'a, T, E> = Pin<Box<dyn Stream<Item = Result<T, E>> + 'a>>;

#[async_trait]
pub trait WebApiPort {
    fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
    ) -> PinBoxStream<'_, AisPosition, QueryError>;
    fn vms_positions(
        &self,
        call_sign: &CallSign,
        range: &DateRange,
    ) -> PinBoxStream<'_, VmsPosition, QueryError>;
    fn species(&self) -> PinBoxStream<'_, Species, QueryError>;
    fn species_fiskeridir(&self) -> PinBoxStream<'_, SpeciesFiskeridir, QueryError>;
    fn species_fao(&self) -> PinBoxStream<'_, SpeciesFao, QueryError>;
    fn vessels(&self) -> Pin<Box<dyn Stream<Item = Result<Vessel, QueryError>> + Send + '_>>;
    fn hauls(&self, query: HaulsQuery) -> Result<PinBoxStream<'_, Haul, QueryError>, QueryError>;
    async fn detailed_trip_of_haul(
        &self,
        haul_id: &HaulId,
    ) -> Result<Option<TripDetailed>, QueryError>;
    async fn hauls_grid(&self, query: HaulsQuery) -> Result<HaulsGrid, QueryError>;
    async fn hauls_matrix(&self, query: HaulsMatrixQuery) -> Result<HaulsMatrix, QueryError>;
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
        id: TripAssemblerId,
    ) -> Result<Vec<TripAssemblerConflict>, QueryError>;
    async fn landing_dates(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
    ) -> Result<Vec<DateTime<Utc>>, QueryError>;
    async fn most_recent_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        assembler_id: TripAssemblerId,
    ) -> Result<Option<Trip>, QueryError>;
    async fn ers_arrivals(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        filter: ArrivalFilter,
    ) -> Result<Vec<Arrival>, QueryError>;
    async fn ers_departures(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
    ) -> Result<Vec<Departure>, QueryError>;
    async fn trip_at_or_prior_to(
        &self,
        vessel_id: FiskeridirVesselId,
        trip_assembler_id: TripAssemblerId,
        time: &DateTime<Utc>,
    ) -> Result<Option<Trip>, QueryError>;
}

#[async_trait]
pub trait TripPrecisionOutboundPort: Send + Sync {
    async fn ports_of_trip(&self, trip_id: TripId) -> Result<TripPorts, QueryError>;
    async fn dock_points_of_trip(&self, trip_id: TripId) -> Result<TripDockPoints, QueryError>;
    async fn ais_positions(
        &self,
        mmsi: Mmsi,
        range: &DateRange,
    ) -> Result<Vec<AisPosition>, QueryError>;
    async fn trip_prior_to(
        &self,
        vessel_id: FiskeridirVesselId,
        assembler_id: TripAssemblerId,
        time: &DateTime<Utc>,
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
