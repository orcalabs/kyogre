use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, FiskeridirVesselId};

use super::{AisVmsPosition, Mmsi, NavigationStatus, PositionType};

#[derive(Debug, Clone)]
pub struct CurrentPositionVessel {
    pub id: FiskeridirVesselId,
    pub mmsi: Option<Mmsi>,
    pub call_sign: Option<CallSign>,
    pub current_trip_start: Option<DateTime<Utc>>,
    pub processing_start: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct CurrentPositionsUpdate {
    pub id: FiskeridirVesselId,
    pub call_sign: Option<CallSign>,
    pub delete_boundary_lower: DateTime<Utc>,
    pub delete_boundary_upper: DateTime<Utc>,
    pub positions: Vec<CurrentPosition>,
}

#[derive(Debug, Clone)]
pub struct CurrentPosition {
    pub vessel_id: FiskeridirVesselId,
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
}

impl CurrentPosition {
    pub fn from_ais_vms(vessel_id: FiskeridirVesselId, pos: AisVmsPosition) -> Self {
        Self {
            vessel_id,
            latitude: pos.latitude,
            longitude: pos.longitude,
            timestamp: pos.timestamp,
            course_over_ground: pos.course_over_ground,
            speed: pos.speed,
            navigational_status: pos.navigational_status,
            rate_of_turn: pos.rate_of_turn,
            true_heading: pos.true_heading,
            distance_to_shore: pos.distance_to_shore,
            position_type: pos.position_type,
        }
    }
}
