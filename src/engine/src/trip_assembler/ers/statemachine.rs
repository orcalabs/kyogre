use super::{ErsEvent, ErsEventType};
use crate::error::Result;
use crate::ers_last_trip_landing_coverage_end;
use chrono::{DateTime, Utc};
use kyogre_core::{Bound, DateRange, NewTrip};

/// Creates trips based on DEP/POR ERS messages where DEP messages indicate the start of a trip
/// and POR messages indicate the end of trip.
/// - If multiple successive DEP messages exist the earliest will be used to indicate trip start.
/// - If multiple successive POR messages exist the latest will be used to indicate trip end.
///
/// Ships are legally required to send DEP/POR messages some hours prior to actually departing or
/// arriving at a port.
/// These messages include an estimated field indicating when the captain thinks they will actually
/// arrive/depart.
/// We use this estimated timestamp to define the ordering of DEP/POR messages and on tiebreaks use
/// ERS message number (num_sent_message within a given year).
///
/// Landing coverage for a trip is defined as (where `POR` is the trip's POR estimated timestamp
/// `POR(N)` is the next trip's POR estimated timestamp): `POR - 6 hours -> POR(N) - 6 hours`
/// - If a trip is shorter than 6 hours `POR` is used as start.
/// - If the next trip is shorter than 6 hours `POR(N)` is used as end.
/// - The latest trip which has no next trip has the end `POR + 3 days`.
///
/// Previously tested approaches for trip definitions:
/// - Sort messages based on ERS message number (num_sent_message within a given year)
///     - Did not work out as their associated timestamp (both estimated and message timestamp) are not ordered correctly.
/// - Latest DEP message instead of earliest
///     - The resulting trips did not accuratley represent fishing trips. There were cases where
///         vessels sent a DEP message, then did some fishing, then sent another DEP and fished some more
///         before sending a POR.
/// - Earliest POR message instead of latest
///     - The resulting trips did not accuratley represent full fishing trips. There were cases where
///         vessels would send multiple successive POR messages at different ports, (deliverying
///         fish at multiple locations) leading to placing landings on wrong trips.
///
/// Previously tested approaches for landing coverage:
/// - `POR -> POR(N)`
///     - We observed that several incidents where landings were registered a short duration prior
///         to POR resulting in them being registered on the prior trip instead of the current trip.
/// - `DEP -> Middle of next trip`
///     - The arbitrary cutoff in the middle of trip resulted in incorrect landing connections, and
///         did not scale well with longer trips.

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
        period.set_equal_end_and_start_to_non_empty();

        let period_extended = DateRange::new(
            self.current_departure
                .estimated_timestamp
                .min(self.current_departure.message_timestamp),
            arrival.estimated_timestamp.max(arrival.message_timestamp),
        )?;

        let mut prior_trip_same_start_and_end_landing_coverage = false;

        if let Some(prior_trip) = self.new_trips.last_mut() {
            let mut range = DateRange::new(
                prior_trip.landing_coverage.start(),
                period.ers_landing_coverage_start(),
            )?;

            if range.equal_start_and_end() {
                range.set_start_bound(Bound::Inclusive);
                range.set_end_bound(Bound::Inclusive);
                prior_trip_same_start_and_end_landing_coverage = true;
            } else if prior_trip.landing_coverage.start_bound() == Bound::Exclusive {
                range.set_start_bound(Bound::Exclusive);
                range.set_end_bound(Bound::Exclusive);
            } else {
                range.set_start_bound(Bound::Inclusive);
                range.set_end_bound(Bound::Exclusive);
            }

            prior_trip.landing_coverage = range;
        }

        let mut landing_coverage = DateRange::new(
            period.ers_landing_coverage_start(),
            ers_last_trip_landing_coverage_end(&period.end()),
        )?;

        if prior_trip_same_start_and_end_landing_coverage {
            landing_coverage.set_start_bound(Bound::Exclusive);
            landing_coverage.set_end_bound(Bound::Exclusive);
        } else {
            landing_coverage.set_start_bound(Bound::Inclusive);
            landing_coverage.set_end_bound(Bound::Exclusive);
        }

        self.new_trips.push(NewTrip {
            period,
            period_extended,
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
