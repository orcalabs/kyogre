use std::fmt::{self, Display};

use crate::*;
use chrono::{DateTime, TimeZone, Utc};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, Quality, VesselLengthGroup};
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

mod assembler;
mod distancer;
mod layer;

pub use assembler::*;
pub use distancer::*;
pub use layer::*;
use strum::{AsRefStr, Display, EnumString};

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
    pub start_port_code: Option<String>,
    pub end_port_code: Option<String>,
    pub target_species_fiskeridir_id: Option<u32>,
    pub target_species_fao_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerLogEntry {
    pub trip_assembler_log_id: u64,
    pub vessel_id: FiskeridirVesselId,
    pub calculation_timer_prior: Option<DateTime<Utc>>,
    pub calculation_timer_post: DateTime<Utc>,
    pub conflict: Option<DateTime<Utc>>,
    pub conflict_vessel_event_timestamp: Option<DateTime<Utc>>,
    pub conflict_vessel_event_id: Option<u64>,
    pub conflict_vessel_event_type_id: Option<VesselEventType>,
    pub conflict_strategy: TripsConflictStrategy,
    pub prior_trip_vessel_events: Vec<MinimalVesselEvent>,
    pub new_vessel_events: Vec<MinimalVesselEvent>,
}

impl TripAssemblerLogEntry {
    pub fn is_conflict(&self) -> bool {
        self.conflict.is_some()
            && self.conflict_vessel_event_timestamp.is_some()
            && self.conflict_vessel_event_type_id.is_some()
            && self.conflict_vessel_event_id.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalVesselEvent {
    pub vessel_event_id: u64,
    pub timestamp: DateTime<Utc>,
    pub event_type: VesselEventType,
}

#[derive(Debug)]
pub struct TripSet {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub conflict_strategy: TripsConflictStrategy,
    pub new_trip_calculation_time: DateTime<Utc>,
    pub trip_assembler_id: TripAssemblerId,
    pub values: Vec<TripProcessingUnit>,
    pub conflict: Option<TripAssemblerConflict>,
    pub new_trip_events: Vec<MinimalVesselEvent>,
    pub prior_trip_events: Vec<MinimalVesselEvent>,
    pub prior_trip_calculation_time: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct TripProcessingUnit {
    pub vessel_id: FiskeridirVesselId,
    pub trip: NewTrip,
    pub trip_assembler_id: TripAssemblerId,
    pub start_port: Option<Port>,
    pub end_port: Option<Port>,
    pub start_dock_points: Vec<PortDockPoint>,
    pub end_dock_points: Vec<PortDockPoint>,
    pub positions: Vec<AisVmsPosition>,
    pub precision_outcome: Option<PrecisionOutcome>,
    pub distance_output: Option<TripDistanceOutput>,
    pub trip_position_output: Option<TripPositionLayerOutput>,
}

#[derive(Debug, Clone)]
pub struct CurrentTrip {
    pub departure: DateTime<Utc>,
    pub target_species_fiskeridir_id: Option<i32>,
    pub hauls: Vec<TripHaul>,
    pub fishing_facilities: Vec<FishingFacility>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTrip {
    pub period: DateRange,
    pub landing_coverage: DateRange,
    pub start_port_code: Option<String>,
    pub end_port_code: Option<String>,
}

#[derive(Debug)]
pub struct TripUpdate {
    pub trip_id: TripId,
    pub precision: Option<PrecisionOutcome>,
    pub distance: Option<TripDistanceOutput>,
    pub position_layers: Option<TripPositionLayerOutput>,
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
    pub fiskeridir_length_group_id: VesselLengthGroup,
    pub trip_id: TripId,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub landing_coverage: DateRange,
    pub num_deliveries: u32,
    pub most_recent_delivery_date: Option<DateTime<Utc>>,
    pub gear_ids: Vec<fiskeridir_rs::Gear>,
    pub gear_group_ids: Vec<fiskeridir_rs::GearGroup>,
    pub species_group_ids: Vec<fiskeridir_rs::SpeciesGroup>,
    pub delivery_point_ids: Vec<DeliveryPointId>,
    pub hauls: Vec<TripHaul>,
    pub fishing_facilities: Vec<FishingFacility>,
    pub delivery: Delivery,
    pub start_port_id: Option<String>,
    pub end_port_id: Option<String>,
    pub assembler_id: TripAssemblerId,
    pub vessel_events: Vec<VesselEvent>,
    pub landing_ids: Vec<LandingId>,
    pub distance: Option<f64>,
    pub cache_version: i64,
    pub target_species_fiskeridir_id: Option<u32>,
    pub target_species_fao_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TripHaul {
    pub haul_id: HaulId,
    pub duration: i32,
    pub ers_activity_id: String,
    pub fiskeridir_vessel_id: Option<i64>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub haul_distance: Option<i32>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub stop_timestamp: DateTime<Utc>,
    pub total_living_weight: i64,
    pub catches: Vec<HaulCatch>,
    pub whale_catches: Vec<WhaleCatch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delivery {
    pub delivered: Vec<Catch>,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    pub product_quality_id: Quality,
    pub product_quality_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString, Display)]
pub enum TripsConflictStrategy {
    #[default]
    Error,
    Replace,
    ReplaceAll,
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
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum TripAssemblerId {
    Landings = 1,
    Ers = 2,
}

impl TripProcessingUnit {
    pub fn start(&self) -> DateTime<Utc> {
        self.trip.period.start()
    }
    pub fn end(&self) -> DateTime<Utc> {
        self.trip.period.end()
    }
    pub fn landing_coverage_start(&self) -> DateTime<Utc> {
        self.trip.landing_coverage.start()
    }
    pub fn landing_coverage_end(&self) -> DateTime<Utc> {
        self.trip.landing_coverage.end()
    }

    pub fn precision_period(&self) -> Option<DateRange> {
        self.precision_outcome.as_ref().and_then(|v| match v {
            PrecisionOutcome::Success {
                new_period,
                start_precision: _,
                end_precision: _,
            } => Some(new_period.clone()),
            PrecisionOutcome::Failed => None,
        })
    }
}

impl From<TripAssemblerId> for i32 {
    fn from(value: TripAssemblerId) -> Self {
        value as i32
    }
}

impl From<&VesselEventDetailed> for MinimalVesselEvent {
    fn from(value: &VesselEventDetailed) -> Self {
        MinimalVesselEvent {
            vessel_event_id: value.event_id,
            timestamp: value.timestamp,
            event_type: value.event_type,
        }
    }
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

impl TripUpdate {
    pub fn precision_period(&self) -> Option<DateRange> {
        self.precision.as_ref().and_then(|v| match v {
            PrecisionOutcome::Success {
                new_period,
                start_precision: _,
                end_precision: _,
            } => Some(new_period.clone()),
            PrecisionOutcome::Failed => None,
        })
    }
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

impl From<PrecisionStatus> for i32 {
    fn from(value: PrecisionStatus) -> Self {
        value as i32
    }
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

impl From<PrecisionDirection> for i32 {
    fn from(value: PrecisionDirection) -> Self {
        value as i32
    }
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
    /// Tries to find positions close to the shore.
    DistanceToShore = 5,
}

impl From<PrecisionId> for i32 {
    fn from(value: PrecisionId) -> Self {
        value as i32
    }
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
    pub queued_reset: bool,
    pub conflict: Option<TripAssemblerConflict>,
}

#[derive(Debug, Clone)]
pub struct TripAssemblerConflict {
    pub timestamp: DateTime<Utc>,
    pub vessel_event_timestamp: DateTime<Utc>,
    pub vessel_event_id: Option<i64>,
    pub event_type: VesselEventType,
}

impl Trip {
    pub fn start(&self) -> DateTime<Utc> {
        self.period.start()
    }
    pub fn end(&self) -> DateTime<Utc> {
        self.period.end()
    }
}

impl From<TripDetailed> for Trip {
    fn from(value: TripDetailed) -> Self {
        Trip {
            trip_id: value.trip_id,
            period: value.period,
            landing_coverage: value.landing_coverage,
            assembler_id: value.assembler_id,
            distance: value.distance,
            precision_period: value.period_precision,
            start_port_code: value.start_port_id,
            end_port_code: value.end_port_id,
            target_species_fiskeridir_id: value.target_species_fiskeridir_id,
            target_species_fao_id: value.target_species_fao_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize_repr)]
#[repr(i32)]
pub enum TripDistancerId {
    AisVms = 1,
}

impl From<TripDistancerId> for i32 {
    fn from(value: TripDistancerId) -> Self {
        value as i32
    }
}

impl std::fmt::Display for TripDistancerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TripDistancerId::AisVms => f.write_str("AisVms"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TripDistanceOutput {
    pub distance: Option<f64>,
    pub distancer_id: TripDistancerId,
}

impl From<i64> for TripId {
    fn from(value: i64) -> Self {
        TripId(value)
    }
}
impl Display for TripId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
