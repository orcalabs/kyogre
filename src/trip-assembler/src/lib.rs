#![deny(warnings)]
#![deny(rust_2018_idioms)]
#![allow(dead_code)]

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use error_stack::Result;
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

// TODO: make this a const when rust supports it
pub fn ers_last_trip_landing_coverage_end() -> DateTime<Utc> {
    DateTime::<Utc>::from_utc(
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2200, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        ),
        Utc,
    )
}

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
        no_prior_state: bool,
    ) -> Result<Vec<NewTrip>, TripAssemblerError>;

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

        let mut new_trips = self
            .new_trips(
                adapter,
                vessel,
                &start,
                matches!(state, State::NoPriorState),
            )
            .await?;
        new_trips.sort_by_key(|n| n.period.end());

        if new_trips.is_empty() {
            Ok(None)
        } else {
            Ok(Some(AssembledTrips {
                new_trip_calculation_time: self.trip_calculation_time(new_trips.last().unwrap()),
                trips: new_trips,
                conflict_strategy: state.conflict_strategy(),
            }))
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssembledTrips {
    pub trips: Vec<NewTrip>,
    pub new_trip_calculation_time: DateTime<Utc>,
    pub conflict_strategy: TripsConflictStrategy,
}

#[derive(Debug, Clone)]
pub enum State {
    Conflict {
        conflict_timestamp: DateTime<Utc>,
        trip_prior_to_or_at_conflict: Option<Trip>,
    },
    CurrentCalculationTime(DateTime<Utc>),
    NoPriorState,
}

impl State {
    fn conflict_strategy(&self) -> TripsConflictStrategy {
        match self {
            State::Conflict {
                conflict_timestamp: _,
                trip_prior_to_or_at_conflict: _,
            }
            | State::NoPriorState => TripsConflictStrategy::Replace,
            State::CurrentCalculationTime(_) => TripsConflictStrategy::Error,
        }
    }
}
