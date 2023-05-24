use chrono::{DateTime, Utc};
use error_stack::Result;
use kyogre_core::{
    Bound, DateRange, DateRangeError, NewTrip, VesselEventDetailed, VesselEventType,
};

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

    pub fn advance(&mut self, event: LandingEvent) -> Result<(), DateRangeError> {
        // We group landing trips per day and want them to end as late as possible to cover
        // all trips for that day.
        if event.timestamp.date_naive() == self.current_landing.timestamp.date_naive() {
            if event.timestamp > self.current_landing.timestamp {
                self.current_landing = event;
            }
        } else {
            let mut period = DateRange::new(self.current_landing.timestamp, event.timestamp)?;
            period.set_start_bound(Bound::Exclusive);
            period.set_end_bound(Bound::Inclusive);
            self.new_trips.push(NewTrip {
                landing_coverage: period.clone(),
                period,
                start_port_code: None,
                end_port_code: None,
            });
            self.current_landing = event;
        }
        Ok(())
    }

    pub fn finalize(self) -> Vec<NewTrip> {
        self.new_trips
    }
}

#[derive(Debug)]
pub struct LandingEvent {
    pub timestamp: DateTime<Utc>,
}

impl LandingEvent {
    pub fn from_vessel_event_detailed(v: VesselEventDetailed) -> Option<LandingEvent> {
        match v.event_type {
            VesselEventType::Landing => Some(LandingEvent {
                timestamp: v.timestamp,
            }),
            VesselEventType::ErsDca => None,
            VesselEventType::ErsPor => None,
            VesselEventType::ErsDep => None,
            VesselEventType::ErsTra => None,
        }
    }

    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
