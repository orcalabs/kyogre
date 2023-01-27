use crate::precision::TripPrecisionCalculator;
use crate::{
    DeliveryPointPrecision, FirstMovedPoint, PrecisionConfig, StartSearchPoint, State,
    TripAssembler, TripAssemblerError, TripPrecisionError,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use error_stack::IntoReport;
use error_stack::{Result, ResultExt};
use kyogre_core::{
    DateRange, NewTrip, PrecisionDirection, Trip, TripAssemblerId, TripAssemblerOutboundPort,
    TripPrecisionOutboundPort, TripPrecisionUpdate, TripsConflictStrategy, Vessel,
};
use std::collections::HashMap;

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
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Landings
    }

    async fn calculate_precision(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripPrecisionOutboundPort,
        trips: Vec<Trip>,
    ) -> Result<Vec<TripPrecisionUpdate>, TripPrecisionError> {
        self.precision_calculator
            .calculate_precision(vessel, adapter, trips)
            .await
    }

    fn start_search_time(&self, state: &State) -> DateTime<Utc> {
        match state {
            State::Conflict(c) => {
                DateTime::<Utc>::from_utc(c.date_naive().and_hms_opt(0, 0, 0).unwrap(), Utc)
            }
            State::CurrentCalculationTime(c) => DateTime::<Utc>::from_utc(
                c.date_naive()
                    .succ_opt()
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
                Utc,
            ),
            State::NoPriorState => Utc.timestamp_opt(1000, 0).unwrap(),
        }
    }

    fn trip_calculation_time(&self, most_recent_trip: &NewTrip) -> DateTime<Utc> {
        DateTime::<Utc>::from_utc(
            most_recent_trip
                .range
                .end()
                .date_naive()
                .and_hms_opt(23, 59, 59)
                .unwrap(),
            Utc,
        )
    }

    async fn new_trips(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        start: &DateTime<Utc>,
        prior_trip: Option<Trip>,
    ) -> Result<(Vec<NewTrip>, Option<TripsConflictStrategy>), TripAssemblerError> {
        let mut landing_dates = adapter
            .landing_dates(vessel.id, start)
            .await
            .change_context(TripAssemblerError)?;
        if landing_dates.is_empty() {
            return Ok((vec![], None));
        }

        let oldest_landing = *landing_dates.iter().min().unwrap();

        let mut new_trips = Vec::new();

        // When conflicts occur on the same day as the prior trip ended we have to add both
        // the start and end of the prior trip to our assembly line as we would lose the link
        // between prior trip if we didnt.
        // If they did not occur on the same day its sufficient to add the end part to our assembly
        // line as this will create the link to the prior trip for our new trips.
        if let Some(prior_trip) = prior_trip {
            if prior_trip.range.end().date_naive() == oldest_landing.date_naive() {
                landing_dates.push(prior_trip.range.start());
            }
            landing_dates.push(prior_trip.range.end());
        } else {
            let end = oldest_landing - Duration::days(1);
            let start = end - Duration::days(1);
            new_trips.push(NewTrip {
                range: DateRange::new(start, end)
                    .into_report()
                    .change_context(TripAssemblerError)?,
                start_port_code: None,
                end_port_code: None,
            });
        };

        let grouped_by_day = group_dates_by_day(landing_dates);

        let mut i = 0;
        let len = grouped_by_day.len();
        while i < len - 1 {
            new_trips.push(NewTrip {
                range: DateRange::new(grouped_by_day[i], grouped_by_day[i + 1])
                    .into_report()
                    .change_context(TripAssemblerError)?,
                start_port_code: None,
                end_port_code: None,
            });

            i += 1;
        }

        Ok((new_trips, None))
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
