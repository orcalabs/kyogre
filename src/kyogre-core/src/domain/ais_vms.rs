use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{DateRange, Mmsi, NavigationStatus, TripId, TripPositionLayerId};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisVmsPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub speed: Option<f64>,
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    pub position_type: PositionType,
    pub pruned_by: Option<TripPositionLayerId>,
    pub trip_cumulative_fuel_consumption: Option<f64>,
    pub trip_cumulative_cargo_weight: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AisVmsPositionWithHaul {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub speed: Option<f64>,
    pub is_inside_haul_and_active_gear: bool,
    pub position_type_id: PositionType,
}

#[derive(Debug, Clone)]
pub struct AisVmsPositionWithHaulAndManual {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub speed: Option<f64>,
    pub is_inside_haul_and_active_gear: bool,
    pub position_type_id: PositionType,
    pub covered_by_manual_fuel_entry: bool,
}

#[derive(Debug, Clone)]
pub enum AisVmsParams {
    Trip(TripId),
    Range {
        mmsi: Option<Mmsi>,
        call_sign: Option<CallSign>,
        range: DateRange,
    },
}

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, Deserialize_repr, Serialize_repr)]
#[repr(i32)]
pub enum PositionType {
    Ais = 1,
    Vms = 2,
}

impl From<PositionType> for i32 {
    fn from(value: PositionType) -> Self {
        value as i32
    }
}

impl From<AisVmsPositionWithHaul> for AisVmsPositionWithHaulAndManual {
    fn from(value: AisVmsPositionWithHaul) -> Self {
        Self {
            speed: value.speed,
            timestamp: value.timestamp,
            position_type_id: value.position_type_id,
            is_inside_haul_and_active_gear: value.is_inside_haul_and_active_gear,
            latitude: value.latitude,
            longitude: value.longitude,
            covered_by_manual_fuel_entry: false,
        }
    }
}
