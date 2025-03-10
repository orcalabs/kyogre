use super::{Vessel, VesselEventDetailed};
use crate::{
    CoreResult, NewTrip, PrecisionOutcome, TripAssemblerConflict, TripAssemblerId,
    TripPrecisionOutboundPort, TripProcessingUnit, TripsConflictStrategy,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use strum::EnumDiscriminants;

#[derive(Debug, Clone, EnumDiscriminants, PartialEq, Eq)]
pub enum AssemblerState {
    Conflict(TripAssemblerConflict),
    NoPriorState,
    Normal(DateTime<Utc>),
    QueuedReset,
}

#[derive(Debug)]
pub struct TripAssemblerState {
    pub new_trips: Vec<NewTrip>,
    pub conflict_strategy: Option<TripsConflictStrategy>,
}

#[async_trait]
pub trait TripAssembler: Send + Sync {
    fn assembler_id(&self) -> TripAssemblerId;
    async fn assemble(
        &self,
        prior_trip_events: Vec<VesselEventDetailed>,
        vessel_events: Vec<VesselEventDetailed>,
    ) -> CoreResult<Option<TripAssemblerState>>;
    async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
    ) -> CoreResult<PrecisionOutcome>;
}
