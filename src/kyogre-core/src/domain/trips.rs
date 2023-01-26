use crate::DateRange;
use chrono::{DateTime, TimeZone, Utc};

#[derive(Debug, Clone)]
pub struct Trip {
    pub trip_id: i64,
    pub range: DateRange,
}

#[derive(Debug, Clone)]
pub struct NewTrip {
    pub range: DateRange,
    pub start_port_code: Option<String>,
    pub end_port_code: Option<String>,
}

#[derive(Debug, Clone)]
pub enum TripsConflictStrategy {
    Error,
}

#[derive(Debug, Clone)]
pub enum TripAssemblerId {
    Landings,
    Ers,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerConflict {
    pub vessel_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripLandingCoverage(DateRange);

impl TripLandingCoverage {
    pub fn start_to_end(range: DateRange) -> TripLandingCoverage {
        TripLandingCoverage(range)
    }

    pub fn start_to_next_start(
        trip_start: DateTime<Utc>,
        next_trip_start: Option<DateTime<Utc>>,
    ) -> Option<TripLandingCoverage> {
        let range = DateRange::new(
            trip_start,
            next_trip_start
                .unwrap_or_else(|| Utc.with_ymd_and_hms(2100, 1, 1, 23, 59, 59).unwrap()),
        )
        .ok()?;
        Some(TripLandingCoverage(range))
    }
}
