use super::{ErsEvent, ErsEventType};
use crate::error::Result;
use crate::ers_last_trip_landing_coverage_end;
use chrono::{DateTime, Utc};
use kyogre_core::{Bound, DateRange, NewTrip};

#[derive(Debug)]
pub struct ErsStatemachine {
    current_departure: Departure,
    current_arrival: Option<Arrival>,
    new_trips: Vec<NewTrip>,
}

impl ErsStatemachine {
    pub fn new(departure: Departure) -> ErsStatemachine {
        ErsStatemachine {
            current_departure: departure,
            current_arrival: None,
            new_trips: vec![],
        }
    }

    fn add_trip(&mut self, arrival: Arrival) -> Result<()> {
        let mut period = DateRange::new(
            self.current_departure.estimated_timestamp,
            arrival.estimated_timestamp,
        )?;

        if period.start() == period.end() {
            period.set_start_bound(Bound::Inclusive);
            period.set_end_bound(Bound::Inclusive);
        } else {
            period.set_start_bound(Bound::Inclusive);
            period.set_end_bound(Bound::Exclusive);
        }

        if let Some(prior_trip) = self.new_trips.last_mut() {
            let mut range = DateRange::new(prior_trip.period.start(), period.start())?;
            range.set_start_bound(Bound::Inclusive);
            range.set_end_bound(Bound::Exclusive);
            prior_trip.landing_coverage = range;
        }

        let mut landing_coverage = DateRange::new(
            period.start(),
            ers_last_trip_landing_coverage_end(&period.end()),
        )?;
        landing_coverage.set_start_bound(Bound::Inclusive);
        landing_coverage.set_end_bound(Bound::Exclusive);

        self.new_trips.push(NewTrip {
            period,
            landing_coverage,
            start_port_code: self.current_departure.port_id.clone(),
            end_port_code: arrival.port_id,
        });

        Ok(())
    }

    pub fn advance(&mut self, event: ErsEvent) -> Result<()> {
        match event.event_type {
            ErsEventType::Arrival => {
                self.current_arrival = Some(Arrival {
                    estimated_timestamp: event.estimated_timestamp,
                    port_id: event.port_id,
                    message_timestamp: event.message_timestamp,
                });
                Ok(())
            }
            ErsEventType::Departure => {
                if let Some(arrival) = self.current_arrival.take() {
                    self.add_trip(arrival)?;
                    self.current_departure = Departure::from(event);
                }
                Ok(())
            }
        }
    }

    pub fn finalize(mut self) -> Result<Vec<NewTrip>> {
        match self.current_arrival.take() {
            Some(arrival) => {
                self.add_trip(arrival)?;
                Ok(self.new_trips)
            }
            None => Ok(self.new_trips),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Departure {
    estimated_timestamp: DateTime<Utc>,
    message_timestamp: DateTime<Utc>,
    port_id: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Arrival {
    estimated_timestamp: DateTime<Utc>,
    message_timestamp: DateTime<Utc>,
    port_id: Option<String>,
}

impl From<ErsEvent> for Arrival {
    fn from(value: ErsEvent) -> Self {
        Arrival {
            estimated_timestamp: value.estimated_timestamp,
            port_id: value.port_id,
            message_timestamp: value.message_timestamp,
        }
    }
}

impl From<ErsEvent> for Departure {
    fn from(value: ErsEvent) -> Self {
        Departure {
            estimated_timestamp: value.estimated_timestamp,
            port_id: value.port_id,
            message_timestamp: value.message_timestamp,
        }
    }
}

impl Departure {
    pub fn from_ers_event(v: ErsEvent) -> Option<Departure> {
        match v.event_type {
            ErsEventType::Arrival => None,
            ErsEventType::Departure => Some(Departure {
                estimated_timestamp: v.estimated_timestamp,
                port_id: v.port_id,
                message_timestamp: v.message_timestamp,
            }),
        }
    }
}
