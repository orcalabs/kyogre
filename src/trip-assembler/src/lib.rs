#![deny(warnings)]
#![deny(rust_2018_idioms)]
#![allow(dead_code)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::{
    NewTrip, Trip, TripAssemblerId, TripAssemblerOutboundPort, TripPrecisionOutboundPort,
    TripPrecisionUpdate, TripsConflictStrategy, Vessel,
};

mod error;
mod ers;
mod landing_assembler;
mod precision;

pub use error::*;
pub use ers::*;
pub use landing_assembler::*;
pub use precision::*;

#[async_trait]
pub trait TripAssembler: Send + Sync {
    fn assembler_id(&self) -> TripAssemblerId;
    fn trip_calculation_time(&self, most_recent_trip: &NewTrip) -> DateTime<Utc>;
    fn start_search_time(&self, state: &State) -> DateTime<Utc>;
    async fn new_trips(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        start: &DateTime<Utc>,
        prior_trip: Option<Trip>,
    ) -> Result<(Vec<NewTrip>, Option<TripsConflictStrategy>), TripAssemblerError>;

    async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trips: Vec<Trip>,
    ) -> Result<Vec<TripPrecisionUpdate>, TripPrecisionError>;

    async fn assemble(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        state: State,
    ) -> Result<Option<AssembledTrips>, TripAssemblerError> {
        let start = self.start_search_time(&state);

        let prior_trip = match state {
            State::Conflict(_) => {
                adapter
                    .trip_prior_to(vessel.id, self.assembler_id(), &start)
                    .await
            }
            State::CurrentCalculationTime(_) => {
                adapter
                    .most_recent_trip(vessel.id, self.assembler_id())
                    .await
            }
            State::NoPriorState => Ok(None),
        }
        .change_context(TripAssemblerError)?;

        let (mut new_trips, conflict_strategy) =
            self.new_trips(adapter, vessel, &start, prior_trip).await?;
        new_trips.sort_by_key(|n| n.range.end());

        if new_trips.is_empty() {
            Ok(None)
        } else {
            Ok(Some(AssembledTrips {
                new_trip_calucation_time: self.trip_calculation_time(new_trips.last().unwrap()),
                trips: new_trips,
                conflict_strategy: conflict_strategy.unwrap_or_else(|| state.conflict_strategy()),
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

#[derive(Debug, Clone)]
pub enum State {
    Conflict(DateTime<Utc>),
    CurrentCalculationTime(DateTime<Utc>),
    NoPriorState,
}

impl State {
    fn conflict_strategy(&self) -> TripsConflictStrategy {
        match self {
            State::Conflict(_) | State::NoPriorState => TripsConflictStrategy::Replace,
            State::CurrentCalculationTime(_) => TripsConflictStrategy::Error,
        }
    }
}
