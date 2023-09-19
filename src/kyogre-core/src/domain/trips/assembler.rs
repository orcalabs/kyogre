use crate::{
    NewTrip, PrecisionOutcome, RelevantEventType, TripAssemblerError, TripAssemblerId,
    TripPrecisionError, TripPrecisionOutboundPort, TripProcessingUnit, TripsConflictStrategy,
    Vessel, VesselEventDetailed,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::Result;

#[derive(Debug, Clone, Copy)]
pub enum AssemblerState {
    Conflict(DateTime<Utc>),
    NoPriorState,
    Normal(DateTime<Utc>),
    QueuedReset,
}

#[derive(Debug)]
pub struct TripAssemblerState {
    pub new_trips: Vec<NewTrip>,
    pub calculation_timer: DateTime<Utc>,
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
    ) -> Result<Option<TripAssemblerState>, TripAssemblerError>;
    async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
    ) -> Result<PrecisionOutcome, TripPrecisionError>;
}
