use crate::trip_assembler::precision::TripPrecisionCalculator;
use crate::{DeliveryPointPrecision, FirstMovedPoint, PrecisionConfig, StartSearchPoint};
use async_trait::async_trait;
use chrono::Duration;
use error_stack::Result;
use error_stack::ResultExt;
use kyogre_core::{
    PrecisionDirection, PrecisionOutcome, RelevantEventType, TripAssembler, TripAssemblerError,
    TripAssemblerId, TripAssemblerState, TripPrecisionError, TripPrecisionOutboundPort,
    TripProcessingUnit, Vessel, VesselEventDetailed,
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
            config,
            PrecisionDirection::Shrinking,
        ));
        LandingTripAssembler {
            precision_calculator: TripPrecisionCalculator::new(vec![start], vec![dp_end, end]),
        }
    }
}

#[async_trait]
impl TripAssembler for LandingTripAssembler {
    fn relevant_event_types(&self) -> RelevantEventType {
        RelevantEventType::Landing
    }
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Landings
    }
    async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trip: &TripProcessingUnit,
    ) -> Result<PrecisionOutcome, TripPrecisionError> {
        self.precision_calculator
            .calculate_precision(vessel, adapter, trip)
            .await
    }

    async fn assemble(
        &self,
        prior_trip_events: Vec<VesselEventDetailed>,
        vessel_events: Vec<VesselEventDetailed>,
    ) -> Result<Option<TripAssemblerState>, TripAssemblerError> {
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

        let start_landing = if prior_trip_events.is_empty() {
            // As we do not have any reasonable estimate of the first trip of a vessel
            // we set it to start a day prior to the first landing.
            LandingEvent {
                timestamp: vessel_events.first().unwrap().timestamp() - Duration::days(1),
            }
        } else {
            // Need to connect the prior trip to the new one
            LandingEvent {
                timestamp: prior_trip_events.last().unwrap().timestamp(),
            }
        };

        let mut state = LandingStatemachine::new(start_landing);

        for e in vessel_events {
            state.advance(e).change_context(TripAssemblerError)?;
        }

        let new_trips = state.finalize();

        if new_trips.is_empty() {
            Ok(None)
        } else {
            Ok(Some(TripAssemblerState {
                calculation_timer: new_trips.last().unwrap().period.end(),
                new_trips,
                conflict_strategy: None,
            }))
        }
    }
}
