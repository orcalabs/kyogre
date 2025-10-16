use crate::error::Result;
use chrono::{DateTime, Duration, Utc};
use kyogre_core::{Bound, DateRange, NewTrip, VesselEventDetailed, VesselEventType};

const MAX_LANDING_TRIP_DURATION: Duration = Duration::days(60);

pub struct LandingStatemachine {
    current_landing: LandingEvent,
    new_trips: Vec<NewTrip>,
}

impl LandingStatemachine {
    pub fn new(landing: LandingEvent) -> LandingStatemachine {
        LandingStatemachine {
            current_landing: landing,
            new_trips: vec![],
        }
    }

    pub fn advance(&mut self, event: LandingEvent) -> Result<()> {
        // We group landing trips per day and want them to end as late as possible to cover
        // all landings for that day.
        if event.timestamp.date_naive() == self.current_landing.timestamp.date_naive() {
            if event.timestamp > self.current_landing.timestamp {
                if !self.new_trips.is_empty() {
                    let idx = self.new_trips.len() - 1;

                    let mut period =
                        DateRange::new(self.new_trips[idx].period.start(), event.timestamp)?;
                    period.set_start_bound(Bound::Exclusive);
                    period.set_end_bound(Bound::Inclusive);

                    self.new_trips[idx].period = period.clone();
                    self.new_trips[idx].landing_coverage = period;
                }

                self.current_landing = event;
            }
        } else {
            let mut period = DateRange::new(self.current_landing.timestamp, event.timestamp)?;
            period.set_start_bound(Bound::Exclusive);
            period.set_end_bound(Bound::Inclusive);

            if period.duration() > MAX_LANDING_TRIP_DURATION {
                period.set_start(period.end() - MAX_LANDING_TRIP_DURATION);
            }

            self.new_trips.push(NewTrip {
                landing_coverage: period.clone(),
                period_extended: period.clone(),
                period,
                start_port_code: None,
                end_port_code: None,
                first_arrival: None,
                start_vessel_event_id: self.current_landing.vessel_event_id.vessel_event_id(),
                // SAFETY: Should always be set.
                end_vessel_event_id: Some(event.vessel_event_id.vessel_event_id().unwrap()),
            });
            self.current_landing = event;
        }
        Ok(())
    }

    pub fn finalize(self) -> Vec<NewTrip> {
        self.new_trips
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LandingEvent {
    pub timestamp: DateTime<Utc>,
    pub vessel_event_id: LandingVesselEventId,
}

#[derive(Debug, Clone, Copy)]
pub enum LandingVesselEventId {
    // For the first ever landing event we create an artifical one that occurs a single day prior
    // which does not have a `vessel_event_id` to link to.
    ArtificalLandingPreceedingFirstLanding,
    RegularLanding { vessel_event_id: i64 },
}

impl LandingVesselEventId {
    fn vessel_event_id(&self) -> Option<i64> {
        match self {
            LandingVesselEventId::ArtificalLandingPreceedingFirstLanding => None,
            LandingVesselEventId::RegularLanding { vessel_event_id } => Some(*vessel_event_id),
        }
    }
}

impl LandingEvent {
    pub fn from_vessel_event_detailed(v: VesselEventDetailed) -> Option<LandingEvent> {
        match v.event_type {
            VesselEventType::Landing => Some(LandingEvent {
                timestamp: v.reported_timestamp,
                vessel_event_id: LandingVesselEventId::RegularLanding {
                    vessel_event_id: v.event_id as i64,
                },
            }),
            VesselEventType::ErsDca => None,
            VesselEventType::ErsPor => None,
            VesselEventType::ErsDep => None,
            VesselEventType::ErsTra => None,
            VesselEventType::Haul => None,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
