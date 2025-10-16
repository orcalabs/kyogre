use crate::trip_assembler::landings::statemachine::LandingVesselEventId;
use crate::trip_assembler::precision::TripPrecisionCalculator;
use crate::{
    DeliveryPointPrecision, DistanceToShorePrecision, FirstMovedPoint, PrecisionConfig,
    StartSearchPoint, error::Result,
};
use async_trait::async_trait;
use chrono::Duration;
use kyogre_core::{
    CoreResult, PrecisionDirection, PrecisionOutcome, TripAssembler, TripAssemblerId,
    TripAssemblerState, TripPrecisionOutboundPort, TripProcessingUnit, TripsConflictStrategy,
    Vessel, VesselEventDetailed,
};

use self::statemachine::{LandingEvent, LandingStatemachine};

mod statemachine;

pub struct LandingTripAssembler {
    precision_calculator: TripPrecisionCalculator,
}

impl LandingTripAssembler {
    pub fn new(precision_calculator: TripPrecisionCalculator) -> LandingTripAssembler {
        LandingTripAssembler {
            precision_calculator,
        }
    }
}

impl Default for LandingTripAssembler {
    fn default() -> Self {
        let config = PrecisionConfig::default();
        let start = Box::new(FirstMovedPoint::new(
            config.clone(),
            StartSearchPoint::Start,
        ));
        let end = Box::new(FirstMovedPoint::new(config.clone(), StartSearchPoint::End));
        let dp_end = Box::new(DeliveryPointPrecision::new(
            config.clone(),
            PrecisionDirection::Shrinking,
        ));
        let distance_to_shore_start = Box::new(DistanceToShorePrecision::new(
            config.clone(),
            PrecisionDirection::Extending,
            StartSearchPoint::Start,
        ));
        let distance_to_shore_end = Box::new(DistanceToShorePrecision::new(
            config,
            PrecisionDirection::Shrinking,
            StartSearchPoint::End,
        ));
        LandingTripAssembler {
            precision_calculator: TripPrecisionCalculator::new(
                vec![start, distance_to_shore_start],
                vec![dp_end, end, distance_to_shore_end],
            ),
        }
    }
}

#[async_trait]
impl TripAssembler for LandingTripAssembler {
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Landings
    }
    async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
    ) -> CoreResult<PrecisionOutcome> {
        Ok(self
            .precision_calculator
            .calculate_precision(vessel, adapter, trip)
            .await?)
    }

    async fn assemble(
        &self,
        prior_trip_events: Vec<VesselEventDetailed>,
        vessel_events: Vec<VesselEventDetailed>,
    ) -> CoreResult<Option<TripAssemblerState>> {
        Ok(assemble_impl(prior_trip_events, vessel_events).await?)
    }
}

async fn assemble_impl(
    prior_trip_events: Vec<VesselEventDetailed>,
    vessel_events: Vec<VesselEventDetailed>,
) -> Result<Option<TripAssemblerState>> {
    let vessel_events: Vec<LandingEvent> = vessel_events
        .into_iter()
        .filter_map(LandingEvent::from_vessel_event_detailed)
        .collect();

    let prior_trip_events: Vec<LandingEvent> = prior_trip_events
        .into_iter()
        .filter_map(LandingEvent::from_vessel_event_detailed)
        .collect();

    if vessel_events.is_empty() {
        return Ok(None);
    }

    let mut conflict_strategy = None;

    let start_landing = if prior_trip_events.is_empty() {
        let event = vessel_events.first().unwrap();
        // This arm occurs in three cases:
        // - First ever run of trip assembler (replacing all does nothing).
        // - There exists a conflict prior to all other landings.
        // - There exists a conflict within the first ever trip of the vessel.
        // For the two latter cases we need to remove the artifical trip we created as the first
        // trip and therefore need to replace all existing trips.
        // If we have conflict that far back in the history we have to re-generate all trips
        // anyway.
        conflict_strategy = Some(TripsConflictStrategy::ReplaceAll);
        // As we do not have any reasonable estimate of the first trip of a vessel
        // we set it to start a day prior to the first landing.
        LandingEvent {
            timestamp: event.timestamp() - Duration::days(1),
            vessel_event_id: LandingVesselEventId::ArtificalLandingPreceedingFirstLanding,
        }
    } else {
        let event = prior_trip_events.last().unwrap();
        // Need to connect the prior trip to the new one
        LandingEvent {
            timestamp: event.timestamp(),
            vessel_event_id: event.vessel_event_id,
        }
    };

    let mut state = LandingStatemachine::new(start_landing);

    for e in vessel_events {
        state.advance(e)?;
    }

    let new_trips = state.finalize();

    if new_trips.is_empty() {
        Ok(None)
    } else {
        Ok(Some(TripAssemblerState {
            new_trips,
            conflict_strategy,
        }))
    }
}
