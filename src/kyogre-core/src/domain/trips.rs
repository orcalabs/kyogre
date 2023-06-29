use std::fmt;

use crate::*;
use chrono::{DateTime, TimeZone, Utc};
use fiskeridir_rs::DeliveryPointId;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct TripId(pub i64);

#[derive(Debug, Clone, PartialEq)]
pub struct Trip {
    pub trip_id: TripId,
    pub period: DateRange,
    pub precision_period: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub distance: Option<f64>,
    pub assembler_id: TripAssemblerId,
}

#[derive(Debug, Clone)]
pub struct CurrentTrip {
    pub departure: DateTime<Utc>,
    pub target_species_fiskeridir_id: Option<i32>,
    pub hauls: Vec<Haul>,
    pub fishing_facilities: Vec<FishingFacility>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTrip {
    pub period: DateRange,
    pub landing_coverage: DateRange,
    pub start_port_code: Option<String>,
    pub end_port_code: Option<String>,
}

impl Trip {
    pub fn precision_start(&self) -> Option<DateTime<Utc>> {
        self.precision_period.as_ref().map(|v| v.start())
    }
    pub fn precision_end(&self) -> Option<DateTime<Utc>> {
        self.precision_period.as_ref().map(|v| v.end())
    }
}

#[derive(Debug, Clone)]
pub struct TripDetailed {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub trip_id: TripId,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub num_deliveries: u32,
    pub most_recent_delivery_date: Option<DateTime<Utc>>,
    pub gear_ids: Vec<fiskeridir_rs::Gear>,
    pub delivery_point_ids: Vec<DeliveryPointId>,
    pub hauls: Vec<Haul>,
    pub fishing_facilities: Vec<FishingFacility>,
    pub delivery: Delivery,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub assembler_id: TripAssemblerId,
    pub vessel_events: Vec<VesselEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Delivery {
    pub delivered: Vec<Catch>,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Catch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    pub product_quality_id: i32,
    pub product_quality_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TripsConflictStrategy {
    Error,
    Replace,
}

#[repr(i32)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    Serialize_repr,
    Deserialize_repr,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
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
        new_period: DateRange,
        start_precision: Option<PrecisionUpdate>,
        end_precision: Option<PrecisionUpdate>,
    },
    Failed,
}

impl PrecisionOutcome {
    pub fn status(&self) -> PrecisionStatus {
        match self {
            PrecisionOutcome::Success {
                new_period: _,
                start_precision: _,
                end_precision: _,
            } => PrecisionStatus::Successful,
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
    Unprocessed = 1,
    /// All enabled precision implementations have been attempted, but failed.
    Attempted = 2,
    /// Enabled precision implementations have been attempted and atleast 1 succeeded.
    Successful = 3,
}

impl PrecisionStatus {
    pub fn name(&self) -> &'static str {
        match self {
            PrecisionStatus::Attempted => "attempted",
            PrecisionStatus::Successful => "successful",
            PrecisionStatus::Unprocessed => "unprocessed",
        }
    }
}

/// What direction a precision implementation have modified a trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrecisionDirection {
    /// The trip has been shrinked.
    Shrinking = 1,
    /// The trip has been extended.
    Extending = 2,
}

impl PrecisionDirection {
    pub fn name(&self) -> &'static str {
        match self {
            PrecisionDirection::Shrinking => "shrinking",
            PrecisionDirection::Extending => "extending",
        }
    }
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

impl From<TripDetailed> for Trip {
    fn from(value: TripDetailed) -> Self {
        Trip {
            trip_id: value.trip_id,
            period: value.period,
            landing_coverage: value.landing_coverage,
            assembler_id: value.assembler_id,
            // TODO
            distance: None,
            precision_period: value.period_precision,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize_repr)]
#[repr(i32)]
pub enum TripDistancerId {
    AisVms = 1,
}

impl std::fmt::Display for TripDistancerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TripDistancerId::AisVms => f.write_str("AisVms"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaulMessage {
    pub message_id: i64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripDistanceOutput {
    pub trip_id: TripId,
    pub distance: Option<f64>,
    pub distancer_id: TripDistancerId,
}
