use chrono::{DateTime, Utc};
use kyogre_core::{NavigationStatus, PositionType, TripPositionLayerId};

use crate::error::PostgresErrorWrapper;

#[derive(Debug, Clone)]
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
    pub position_type_id: PositionType,
    pub pruned_by: Option<TripPositionLayerId>,
}

impl TryFrom<AisVmsPosition> for kyogre_core::AisVmsPosition {
    type Error = PostgresErrorWrapper;

    fn try_from(v: AisVmsPosition) -> Result<Self, Self::Error> {
        Ok(Self {
            latitude: v.latitude,
            longitude: v.longitude,
            timestamp: v.timestamp,
            course_over_ground: v.course_over_ground,
            speed: v.speed,
            navigational_status: v.navigational_status,
            rate_of_turn: v.rate_of_turn,
            true_heading: v.true_heading,
            distance_to_shore: v.distance_to_shore,
            position_type: v.position_type_id,
            pruned_by: v.pruned_by,
        })
    }
}
