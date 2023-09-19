use super::{
    ers::statemachine::ErsStatemachine, precision::TripPrecisionCalculator, DeliveryPointPrecision,
    DockPointPrecision, PortPrecision, PrecisionConfig, StartSearchPoint,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::{
    FiskeridirVesselId, PrecisionDirection, PrecisionOutcome, RelevantEventType, TripAssembler,
    TripAssemblerError, TripAssemblerId, TripAssemblerState, TripPrecisionError,
    TripPrecisionOutboundPort, TripProcessingUnit, TripsConflictStrategy, Vessel, VesselEventData,
    VesselEventDetailed,
};

use self::statemachine::Departure;

mod statemachine;

pub struct ErsTripAssembler {
    precision_calculator: TripPrecisionCalculator,
}

impl Default for ErsTripAssembler {
    fn default() -> Self {
        let config = PrecisionConfig::default();
        let dp_end = Box::new(DeliveryPointPrecision::new(
            config.clone(),
            PrecisionDirection::Extending,
        ));
        let port_start = Box::new(PortPrecision::new(
            config.clone(),
            PrecisionDirection::Extending,
            StartSearchPoint::Start,
        ));
        let port_end = Box::new(PortPrecision::new(
            config.clone(),
            PrecisionDirection::Extending,
            StartSearchPoint::End,
        ));
        let dock_point_start = Box::new(DockPointPrecision::new(
            config.clone(),
            PrecisionDirection::Extending,
            StartSearchPoint::Start,
        ));
        let dock_point_end = Box::new(DockPointPrecision::new(
            config,
            PrecisionDirection::Extending,
            StartSearchPoint::End,
        ));
        ErsTripAssembler {
            precision_calculator: TripPrecisionCalculator::new(
                vec![port_start, dock_point_start],
                vec![dp_end, port_end, dock_point_end],
            ),
        }
    }
}

impl ErsTripAssembler {
    pub fn new(precision_calculator: TripPrecisionCalculator) -> ErsTripAssembler {
        ErsTripAssembler {
            precision_calculator,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ErsEvent {
    event_id: u64,
    vessel_id: FiskeridirVesselId,
    message_timestamp: DateTime<Utc>,
    estimated_timestamp: DateTime<Utc>,
    event_type: ErsEventType,
    port_id: Option<String>,
}

#[derive(Debug, Clone)]
enum ErsEventType {
    Arrival,
    Departure,
}

impl ErsEvent {
    fn from_detailed_vessel_event(v: VesselEventDetailed) -> Option<ErsEvent> {
        match v.event_data {
            VesselEventData::ErsDep {
                port_id,
                estimated_timestamp,
            } => Some((ErsEventType::Departure, port_id, estimated_timestamp)),
            VesselEventData::ErsPor {
                port_id,
                estimated_timestamp,
            } => Some((ErsEventType::Arrival, port_id, estimated_timestamp)),
            VesselEventData::Landing => None,
            VesselEventData::ErsDca => None,
            VesselEventData::ErsTra => None,
            VesselEventData::Haul => None,
        }
        .map(|(event_type, port_id, estimated_timestamp)| ErsEvent {
            event_id: v.event_id,
            vessel_id: v.vessel_id,
            message_timestamp: v.timestamp,
            event_type,
            port_id,
            estimated_timestamp,
        })
    }
}

#[async_trait]
impl TripAssembler for ErsTripAssembler {
    fn relevant_event_types(&self) -> RelevantEventType {
        RelevantEventType::ErsPorAndDep
    }
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Ers
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
        let mut conflict_strategy = None;
        let mut vessel_events: Vec<ErsEvent> = vessel_events
            .into_iter()
            .filter_map(ErsEvent::from_detailed_vessel_event)
            .collect();

        let mut prior_trip_events: Vec<ErsEvent> = prior_trip_events
            .into_iter()
            .filter_map(ErsEvent::from_detailed_vessel_event)
            .collect();

        if vessel_events.is_empty() {
            return Ok(None);
        }

        // We need to handle the case where the first ever message from a vessel is an arrival
        let (events_to_process, mut state) = if prior_trip_events.is_empty() {
            let mut departure_index = None;
            for (i, e) in vessel_events.iter().enumerate() {
                match e.event_type {
                    ErsEventType::Arrival => {
                        continue;
                    }
                    ErsEventType::Departure => {
                        departure_index = Some(i);
                        break;
                    }
                }
            }

            if let Some(idx) = departure_index {
                let state = ErsStatemachine::new(
                    Departure::from_ers_event(vessel_events[idx].clone()).unwrap(),
                );
                Ok((vessel_events[idx..].to_vec(), state))
            } else {
                return Ok(None);
            }
        } else {
            let current_event = vessel_events.remove(0);
            match current_event.event_type {
                ErsEventType::Arrival => {
                    prior_trip_events.push(current_event);
                    prior_trip_events.append(&mut vessel_events);
                    conflict_strategy = Some(TripsConflictStrategy::Replace);

                    let current_event = prior_trip_events.remove(0);
                    match current_event.event_type {
                        ErsEventType::Arrival => Err(TripStartedOnArrivalError(current_event)),
                        ErsEventType::Departure => Ok((
                            prior_trip_events,
                            ErsStatemachine::new(Departure::from_ers_event(current_event).unwrap()),
                        )),
                    }
                }
                ErsEventType::Departure => Ok((
                    vessel_events,
                    ErsStatemachine::new(Departure::from_ers_event(current_event).unwrap()),
                )),
            }
        }
        .change_context(TripAssemblerError)?;

        for e in events_to_process {
            state.advance(e).change_context(TripAssemblerError)?;
        }

        let new_trips = state.finalize().change_context(TripAssemblerError)?;

        if new_trips.is_empty() {
            Ok(None)
        } else {
            Ok(Some(TripAssemblerState {
                calculation_timer: new_trips.last().unwrap().period.end(),
                new_trips,
                conflict_strategy,
            }))
        }
    }
}

#[derive(Debug)]
struct TripStartedOnArrivalError(ErsEvent);

impl std::error::Error for TripStartedOnArrivalError {}

impl std::fmt::Display for TripStartedOnArrivalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "an ers based trip started on an arrival: {:?}",
            self.0
        ))
    }
}
