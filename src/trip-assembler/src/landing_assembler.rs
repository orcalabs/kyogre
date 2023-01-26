use error_stack::IntoReport;
use std::collections::HashMap;

use crate::{TripAssembler, TripAssemblerError};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::{DateRange, NewTrip, Trip, TripAssemblerId, TripAssemblerOutboundPort, Vessel};

pub struct LandingTripAssembler {}

#[async_trait]
impl TripAssembler for LandingTripAssembler {
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Landings
    }

    async fn new_trips(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        range: &DateRange,
        prior_trip: Option<Trip>,
    ) -> Result<Vec<NewTrip>, TripAssemblerError> {
        let landing_dates = adapter
            .landing_dates(vessel.id, range)
            .await
            .change_context(TripAssemblerError)?;

        let mut grouped_by_day = group_dates_by_day(landing_dates);

        let mut new_trips = Vec::new();
        let len = grouped_by_day.len();

        // If a new landing occurs earlier on the same day as the prior trip's end/start we need to include
        // them in the same day grouping.
        if let (Some(prior_trip), false) = (prior_trip, grouped_by_day.is_empty()) {
            let first_landing_date = grouped_by_day[0].date_naive();
            let prior_trip_start = prior_trip.range.start();
            let prior_trip_end = prior_trip.range.end();

            match (
                prior_trip_start.date_naive() == first_landing_date,
                prior_trip_end.date_naive() == first_landing_date,
            ) {
                // We are only interested in the earlierst landing on each day, as start is always
                // before end we prioritize it.
                (true, _) => grouped_by_day[0] = *prior_trip_start,
                (false, true) => grouped_by_day[0] = *prior_trip_end,
                _ => (),
            }
        }

        let mut i = 0;

        while i < len - 1 {
            let range = DateRange::new(grouped_by_day[i], grouped_by_day[i + 1])
                .into_report()
                .change_context(TripAssemblerError)?;

            let new_trip = NewTrip {
                range,
                start_port_code: None,
                end_port_code: None,
            };

            new_trips.push(new_trip);

            i += 1;
        }

        Ok(new_trips)
    }
}

fn group_dates_by_day(dates: Vec<DateTime<Utc>>) -> Vec<DateTime<Utc>> {
    let mut days: HashMap<NaiveDate, DateTime<Utc>> = HashMap::new();

    for d in dates {
        if let Some(date) = days.get(&d.date_naive()) {
            if d > *date {
                days.insert(d.date_naive(), d);
            }
        } else {
            days.insert(d.date_naive(), d);
        }
    }

    let mut days: Vec<DateTime<Utc>> = days.into_values().collect();

    days.sort();

    days
}
