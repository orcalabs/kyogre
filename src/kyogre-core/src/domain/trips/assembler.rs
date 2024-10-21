use crate::{
    CoreResult, NewTrip, PrecisionOutcome, RelevantEventType, TripAssemblerConflict,
    TripAssemblerId, TripPrecisionOutboundPort, TripProcessingUnit, TripsConflictStrategy, Vessel,
    VesselEventDetailed,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use strum::EnumDiscriminants;

#[derive(Debug, Clone, EnumDiscriminants)]
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
    fn relevant_event_types(&self) -> RelevantEventType;
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
