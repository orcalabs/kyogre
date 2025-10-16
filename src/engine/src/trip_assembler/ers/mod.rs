use crate::{DistanceToShorePrecision, error::error::TripStartedOnArrivalSnafu};

use super::{
    DeliveryPointPrecision, DockPointPrecision, PortPrecision, PrecisionConfig, StartSearchPoint,
    ers::statemachine::ErsStatemachine, precision::TripPrecisionCalculator,
};
use crate::error::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kyogre_core::{
    CoreResult, FiskeridirVesselId, PrecisionDirection, PrecisionOutcome, TripAssembler,
    TripAssemblerId, TripAssemblerState, TripPrecisionOutboundPort, TripProcessingUnit,
    TripsConflictStrategy, Vessel, VesselEventData, VesselEventDetailed,
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
            config.clone(),
            PrecisionDirection::Extending,
            StartSearchPoint::End,
        ));
        let distance_to_shore_start = Box::new(DistanceToShorePrecision::new(
            config.clone(),
            PrecisionDirection::Extending,
            StartSearchPoint::Start,
        ));
        let distance_to_shore_end = Box::new(DistanceToShorePrecision::new(
            config,
            PrecisionDirection::Extending,
            StartSearchPoint::End,
        ));
        ErsTripAssembler {
            precision_calculator: TripPrecisionCalculator::new(
                vec![port_start, dock_point_start, distance_to_shore_start],
                vec![dp_end, port_end, dock_point_end, distance_to_shore_end],
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
    vessel_event_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
            message_timestamp: v.reported_timestamp,
            event_type,
            port_id,
            estimated_timestamp,
            vessel_event_id: v.event_id as i64,
        })
    }
}

#[async_trait]
impl TripAssembler for ErsTripAssembler {
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Ers
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
        // There is nothing to do without any new Por events
        // Only Dep messages will not generate/extend any trips.
        if vessel_events
            .iter()
            .all(|v| v.event_type == ErsEventType::Departure)
        {
            return Ok(None);
        }

        // Adding a new trip or extending a trip will always require recalculating the previous
        // trip due to landing coverage semantics.
        conflict_strategy = Some(TripsConflictStrategy::Replace {
            conflict: prior_trip_events.last().unwrap().estimated_timestamp,
        });
        prior_trip_events.append(&mut vessel_events);

        let current_event = prior_trip_events.remove(0);
        match current_event.event_type {
            ErsEventType::Arrival => TripStartedOnArrivalSnafu {
                event: current_event,
            }
            .fail(),
            ErsEventType::Departure => Ok((
                prior_trip_events,
                ErsStatemachine::new(Departure::from_ers_event(current_event).unwrap()),
            )),
        }
    }?;

    for e in events_to_process {
        state.advance(e)?;
    }

    let new_trips = state.finalize()?;

    if new_trips.is_empty() {
        Ok(None)
    } else {
        Ok(Some(TripAssemblerState {
            new_trips,
            conflict_strategy,
        }))
    }
}
