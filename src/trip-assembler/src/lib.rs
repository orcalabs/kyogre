use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{
    DateRange, NewTrip, Trip, TripAssemblerId, TripAssemblerOutboundPort, TripsConflictStrategy,
    Vessel,
};

mod error;
mod ers;
mod landing_assembler;

pub use error::*;
pub use ers::*;
pub use landing_assembler::*;

#[async_trait]
pub trait TripAssembler {
    fn assembler_id(&self) -> TripAssemblerId;
    async fn new_trips(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        range: &DateRange,
        prior_trip: Option<Trip>,
    ) -> Result<Vec<NewTrip>, TripAssemblerError>;

    async fn assemble(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        current_time: &DateTime<Utc>,
        state: State,
    ) -> Result<Option<AssembledTrips>, TripAssemblerError> {
        let start = match state {
            State::Conflict(c) | State::CurrentCalculationTime(c) => c,
            State::NoPriorState => Utc.timestamp_opt(1000, 0).unwrap(),
        };

        let prior_trip = adapter
            .most_recent_trip(vessel.id, self.assembler_id())
            .await
            .change_context(TripAssemblerError)?;

        let range = DateRange::new(start, *current_time)
            .into_report()
            .change_context(TripAssemblerError)?;

        let new_trips = self.new_trips(adapter, vessel, &range, prior_trip).await?;

        if new_trips.is_empty() {
            Ok(None)
        } else {
            Ok(Some(AssembledTrips {
                trips: new_trips,
                new_trip_calucation_time: *current_time,
                conflict_strategy: TripsConflictStrategy::Error,
            }))
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssembledTrips {
    pub trips: Vec<NewTrip>,
    pub new_trip_calucation_time: DateTime<Utc>,
    pub conflict_strategy: TripsConflictStrategy,
}

#[derive(Debug)]
pub enum State {
    Conflict(DateTime<Utc>),
    CurrentCalculationTime(DateTime<Utc>),
    NoPriorState,
}
