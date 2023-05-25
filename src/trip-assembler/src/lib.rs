#![deny(warnings)]
#![deny(rust_2018_idioms)]

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{
    Bound, FiskeridirVesselId, NewTrip, QueryRange, RelevantEventType, Trip, TripAssemblerId,
    TripAssemblerOutboundPort, TripPrecisionOutboundPort, TripPrecisionUpdate,
    TripsConflictStrategy, Vessel, VesselEventDetailed,
};
use tracing::{event, Level};

mod error;
mod ers;
mod landings;
mod precision;

pub use error::*;
pub use ers::*;
pub use landings::*;
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

#[derive(Debug)]
pub struct TripAssemblerState {
    pub new_trips: Vec<NewTrip>,
    pub calculation_timer: DateTime<Utc>,
    pub conflict_strategy: Option<TripsConflictStrategy>,
}

#[derive(Debug)]
pub struct TripsReport {
    pub num_trips: u32,
    pub num_conflicts: u32,
    pub num_no_prior_state: u32,
    pub num_vessels: u32,
    pub num_failed: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum AssemblerState {
    Conflict(DateTime<Utc>),
    NoPriorState,
    Normal(DateTime<Utc>),
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
        trips: Vec<Trip>,
    ) -> Result<Vec<TripPrecisionUpdate>, TripPrecisionError>;
    async fn produce_and_store_trips(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
    ) -> Result<TripsReport, TripAssemblerError> {
        let relevant_event_types = self.relevant_event_types();
        let timers = adapter
            .trip_calculation_timers(self.assembler_id())
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|v| (v.fiskeridir_vessel_id, v.timestamp))
            .collect::<HashMap<FiskeridirVesselId, DateTime<Utc>>>();

        let conflicts = adapter
            .conflicts(self.assembler_id())
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|v| (v.fiskeridir_vessel_id, v.timestamp))
            .collect::<HashMap<FiskeridirVesselId, DateTime<Utc>>>();

        let vessels = adapter
            .all_vessels()
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|v| (v.fiskeridir.id, v))
            .collect::<HashMap<FiskeridirVesselId, Vessel>>();

        let num_vessels = vessels.len() as u32;
        let mut num_conflicts = 0;
        let mut num_no_prior_state = 0;
        let mut num_trips = 0;
        let mut num_failed = 0;

        for v in vessels.into_values() {
            if v.preferred_trip_assembler != self.assembler_id() {
                continue;
            }
            let vessel_id = v.fiskeridir.id;
            let state = match (timers.get(&vessel_id), conflicts.get(&vessel_id)) {
                (None, None) => AssemblerState::NoPriorState,
                (None, Some(t)) => AssemblerState::Conflict(*t),
                (Some(t), None) => AssemblerState::Normal(*t),
                (Some(_), Some(t)) => AssemblerState::Conflict(*t),
            };

            let vessel_events = match state {
                AssemblerState::Conflict(t) => {
                    num_conflicts += 1;
                    new_vessel_events(
                        vessel_id,
                        adapter,
                        relevant_event_types,
                        &t,
                        Bound::Exclusive,
                    )
                    .await
                }
                AssemblerState::Normal(t) => {
                    new_vessel_events(
                        vessel_id,
                        adapter,
                        relevant_event_types,
                        &t,
                        Bound::Inclusive,
                    )
                    .await
                }
                AssemblerState::NoPriorState => {
                    num_no_prior_state += 1;
                    all_vessel_events(vessel_id, adapter, relevant_event_types).await
                }
            }?;
            let trips = self
                .assemble(
                    vessel_events.prior_trip_events,
                    vessel_events.new_vessel_events,
                )
                .await?;

            if let Some(trips) = trips {
                let conflict_strategy = match (state, trips.conflict_strategy) {
                    (AssemblerState::NoPriorState, Some(r))
                    | (AssemblerState::Normal(_), Some(r)) => r,
                    (AssemblerState::NoPriorState, None) | (AssemblerState::Normal(_), None) => {
                        TripsConflictStrategy::Error
                    }
                    (AssemblerState::Conflict(_), _) => TripsConflictStrategy::Replace,
                };
                num_trips += trips.new_trips.len() as u32;
                if let Err(e) = adapter
                    .add_trips(
                        vessel_id,
                        trips.calculation_timer,
                        conflict_strategy,
                        trips.new_trips,
                        self.assembler_id(),
                    )
                    .await
                    .change_context(TripAssemblerError)
                {
                    num_failed += 1;
                    event!(
                        Level::ERROR,
                        "failed to store trips for vessel_id: {}, err: {:?}",
                        vessel_id.0,
                        e
                    );
                }
            }
        }

        Ok(TripsReport {
            num_conflicts,
            num_vessels,
            num_no_prior_state,
            num_trips,
            num_failed,
        })
    }
}

#[derive(Debug)]
struct VesselEvents {
    prior_trip_events: Vec<VesselEventDetailed>,
    new_vessel_events: Vec<VesselEventDetailed>,
}

async fn new_vessel_events(
    vessel_id: FiskeridirVesselId,
    adapter: &dyn TripAssemblerOutboundPort,
    relevant_event_types: RelevantEventType,
    search_timestamp: &DateTime<Utc>,
    bound: Bound,
) -> Result<VesselEvents, TripAssemblerError> {
    let prior_trip = adapter
        .trip_prior_to_timestamp(vessel_id, search_timestamp, bound)
        .await
        .change_context(TripAssemblerError)?;

    let res: Result<(Vec<VesselEventDetailed>, QueryRange), TripAssemblerError> = match prior_trip {
        Some(prior_trip) => {
            let range = QueryRange::new(
                match prior_trip.period.end_bound() {
                    // We want all events not covered by the trip and therefore swap the bounds
                    kyogre_core::Bound::Inclusive => std::ops::Bound::Excluded(prior_trip.end()),
                    kyogre_core::Bound::Exclusive => std::ops::Bound::Included(prior_trip.end()),
                },
                std::ops::Bound::Unbounded,
            )
            .into_report()
            .change_context(TripAssemblerError)?;

            let events = adapter
                .relevant_events(
                    vessel_id,
                    &QueryRange::from(prior_trip.period),
                    relevant_event_types,
                )
                .await
                .change_context(TripAssemblerError)?;

            Ok((events, range))
        }
        None => {
            let range = QueryRange::new(
                std::ops::Bound::Included(*search_timestamp),
                std::ops::Bound::Unbounded,
            )
            .into_report()
            .change_context(TripAssemblerError)?;

            Ok((vec![], range))
        }
    };

    let (prior_trip_events, new_events_search_range) = res?;

    let new_vessel_events = adapter
        .relevant_events(vessel_id, &new_events_search_range, relevant_event_types)
        .await
        .change_context(TripAssemblerError)?;

    Ok(VesselEvents {
        prior_trip_events,
        new_vessel_events,
    })
}

async fn all_vessel_events(
    vessel_id: FiskeridirVesselId,
    adapter: &dyn TripAssemblerOutboundPort,
    relevant_event_types: RelevantEventType,
) -> Result<VesselEvents, TripAssemblerError> {
    let range = QueryRange::new(std::ops::Bound::Unbounded, std::ops::Bound::Unbounded)
        .into_report()
        .change_context(TripAssemblerError)?;

    let new_vessel_events = adapter
        .relevant_events(vessel_id, &range, relevant_event_types)
        .await
        .change_context(TripAssemblerError)?;

    Ok(VesselEvents {
        prior_trip_events: vec![],
        new_vessel_events,
    })
}
