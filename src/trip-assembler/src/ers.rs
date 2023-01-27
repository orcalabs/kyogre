use crate::{
    precision::TripPrecisionCalculator, State, TripAssembler, TripAssemblerError,
    TripPrecisionError,
};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use error_stack::{IntoReport, Result, ResultExt};
use kyogre_core::{
    Arrival, ArrivalFilter, DateRange, Departure, NewTrip, Trip, TripAssemblerId,
    TripAssemblerOutboundPort, TripPrecisionOutboundPort, TripPrecisionUpdate,
    TripsConflictStrategy, Vessel,
};
use strum::EnumDiscriminants;

pub struct ErsTripAssembler {
    precision_calculator: TripPrecisionCalculator,
}

impl ErsTripAssembler {
    pub fn new(precision_calculator: TripPrecisionCalculator) -> ErsTripAssembler {
        ErsTripAssembler {
            precision_calculator,
        }
    }
}

#[derive(EnumDiscriminants, Debug, Eq)]
enum StopPoint {
    Arrival(Arrival),
    Departure(Departure),
}

impl StopPoint {
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            StopPoint::Arrival(a) => a.timestamp,
            StopPoint::Departure(d) => d.timestamp,
        }
    }
    pub fn port_code(&self) -> Option<&str> {
        match self {
            StopPoint::Arrival(a) => a.port_code.as_deref(),
            StopPoint::Departure(d) => d.port_code.as_deref(),
        }
    }
}

impl PartialEq for StopPoint {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp().eq(&other.timestamp())
    }
}

impl PartialOrd for StopPoint {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.timestamp().partial_cmp(&other.timestamp())
    }
}

impl Ord for StopPoint {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp().cmp(&other.timestamp())
    }
}

#[async_trait]
impl TripAssembler for ErsTripAssembler {
    fn assembler_id(&self) -> TripAssemblerId {
        TripAssemblerId::Ers
    }

    fn start_search_time(&self, state: &State) -> DateTime<Utc> {
        match state {
            State::Conflict(c) | State::CurrentCalculationTime(c) => *c,
            State::NoPriorState => Utc.timestamp_opt(1000, 0).unwrap(),
        }
    }

    fn trip_calculation_time(&self, most_recent_trip: &NewTrip) -> DateTime<Utc> {
        most_recent_trip.range.end()
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

    async fn new_trips(
        &self,
        adapter: &dyn TripAssemblerOutboundPort,
        vessel: &Vessel,
        start: &DateTime<Utc>,
        prior_trip: Option<Trip>,
    ) -> Result<(Vec<NewTrip>, Option<TripsConflictStrategy>), TripAssemblerError> {
        let mut stop_points: Vec<StopPoint> = Vec::new();
        let mut conflict_strategy = None;

        // If a new arrival is added and no further departures, we want to extend the current trip to
        // that next arrival.
        if let Some(p) = &prior_trip {
            let prior_departure = adapter
                .departure_of_trip(p.trip_id)
                .await
                .change_context(TripAssemblerError)?;

            stop_points.push(StopPoint::Departure(prior_departure));
        }

        let arrivals = adapter
            .ers_arrivals(vessel.id, start, ArrivalFilter::WithLandingFacility)
            .await
            .change_context(TripAssemblerError)
            .into_iter()
            .map(StopPoint::Arrival);
        let departures = adapter
            .ers_departures(vessel.id, start)
            .await
            .change_context(TripAssemblerError)
            .into_iter()
            .map(StopPoint::Departure);

        stop_points.extend(arrivals);
        stop_points.extend(departures);
        stop_points.sort();
        // There might be duplicates if there was a conflict.
        stop_points.dedup();

        let mut new_trips = Vec::new();
        let len = stop_points.len();

        let mut current_departure_threshold = 0;

        for (i, current_stop) in stop_points.iter().enumerate() {
            match i {
                i if (i != 0 && i != len - 1) || (i == 0 && len != 1) => {
                    let current_stop_type = StopPointDiscriminants::from(current_stop);
                    if current_stop_type != StopPointDiscriminants::Departure
                        || i < current_departure_threshold
                    {
                        continue;
                    }

                    if let Some(arrival) = find_arrival_preceding_next_departure(i, &stop_points) {
                        current_departure_threshold = arrival.0;
                        let arrival = arrival.1;

                        let range = DateRange::new(current_stop.timestamp(), arrival.timestamp)
                            .into_report()
                            .change_context(TripAssemblerError)?;

                        new_trips.push(NewTrip {
                            range,
                            start_port_code: current_stop.port_code().map(|p| p.to_string()),
                            end_port_code: arrival.port_code.clone(),
                        });
                    } else {
                        break;
                    }
                }
                _ => {}
            }
        }

        if !new_trips.is_empty() {
            // Since we include the prior departure we will re-create the most recent trip each
            // assembly round.
            // If the most recent trip has not changed (a new arrival extending it has not
            // occurred) we do not need to re-add it.
            if let Some(prior_trip) = prior_trip {
                if new_trips[0].range.start().timestamp() == prior_trip.range.start().timestamp()
                    && new_trips[0].range.end().timestamp() == prior_trip.range.end().timestamp()
                {
                    new_trips.remove(0);
                } else {
                    conflict_strategy = Some(TripsConflictStrategy::Replace);
                }
            }
        }

        Ok((new_trips, conflict_strategy))
    }
}

fn find_arrival_preceding_next_departure(
    current_index: usize,
    points: &[StopPoint],
) -> Option<(usize, &Arrival)> {
    let mut i = current_index;

    let len = points.len();

    while i < len {
        match &points[i] {
            StopPoint::Arrival(a) => {
                if i == len - 1 || matches!(&points[i + 1], StopPoint::Departure(_)) {
                    return Some((i, a));
                }
            }
            StopPoint::Departure(_) => (),
        }
        i += 1;
    }

    None
}
