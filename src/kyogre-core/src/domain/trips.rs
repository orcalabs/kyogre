use crate::{DateRange, FiskeridirVesselId, Port};
use chrono::{DateTime, TimeZone, Utc};
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct TripId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trip {
    pub trip_id: TripId,
    pub period: DateRange,
    pub landing_coverage: DateRange,
    pub assembler_id: TripAssemblerId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTrip {
    pub period: DateRange,
    pub start_port_code: Option<String>,
    pub end_port_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TripsConflictStrategy {
    Error,
    Replace,
}

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
pub enum TripAssemblerId {
    Landings = 1,
    Ers = 2,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerConflict {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TripLandingCoverage(DateRange);

/// An outcome of exectuing one or more trip precision implementations on a trip.
#[derive(Debug, Clone)]
pub struct TripPrecisionUpdate {
    pub trip_id: TripId,
    pub outcome: PrecisionOutcome,
}

#[derive(Debug, Clone)]
pub enum PrecisionOutcome {
    Success {
        new_range: DateRange,
        start_precision: Option<PrecisionUpdate>,
        end_precision: Option<PrecisionUpdate>,
    },
    Failed,
}

impl PrecisionOutcome {
    pub fn status(&self) -> PrecisionStatus {
        match self {
            PrecisionOutcome::Success {
                new_range: _,
                start_precision: _,
                end_precision: _,
            } => PrecisionStatus::Completed,
            PrecisionOutcome::Failed => PrecisionStatus::Attempted,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrecisionUpdate {
    pub id: PrecisionId,
    pub direction: PrecisionDirection,
}

/// Status of the outcome of all enabled precision implementations for a given trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrecisionStatus {
    /// No precision implementation has been attempted.
    Original = 1,
    /// All enabled precision implementations have been attempted, but failed.
    Attempted = 2,
    /// Enabled precision implementations have been attempted and atleast 1 succeeded.
    Completed = 3,
}

/// What direction a precision implementation have modified a trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrecisionDirection {
    /// The trip has been shrinked.
    Shrinking = 1,
    /// The trip has been extended.
    Extending = 2,
}

/// All trip precision implementations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrecisionId {
    /// Tries to find the first moved point.
    FirstMovedPoint = 1,
    /// Tries to find positions close to the delivery points associated with the trip.
    DeliveryPoint = 2,
    /// Tries to find positions close to the ports associated with the trip.
    Port = 3,
    /// Tries to find positions close to the dock points associated with the trip.
    DockPoint = 4,
}

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

/// Convenience type for ports associated with a trip.
#[derive(Debug, Clone, PartialEq)]
pub struct TripPorts {
    pub start: Option<Port>,
    pub end: Option<Port>,
}

#[derive(Debug, Clone)]
pub struct TripCalculationTimer {
    pub timestamp: DateTime<Utc>,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
}

impl Trip {
    pub fn start(&self) -> DateTime<Utc> {
        self.period.start()
    }
    pub fn end(&self) -> DateTime<Utc> {
        self.period.end()
    }
}

impl std::fmt::Display for TripAssemblerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TripAssemblerId::Landings => write!(f, "landings_assembler"),
            TripAssemblerId::Ers => write!(f, "ers_assembler"),
        }
    }
}
