use chrono::{DateTime, Utc};
use kyogre_core::{FiskeridirVesselId, NavigationStatus, PositionType};
use unnest_insert::UnnestInsert;

use crate::queries::{opt_type_to_i32, type_to_i32, type_to_i64};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "current_trip_positions",
    conflict = "fiskeridir_vessel_id,position_type_id,timestamp"
)]
pub struct CurrentPosition {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub speed: Option<f64>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub navigation_status_id: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub position_type_id: PositionType,
}

impl From<&kyogre_core::CurrentPosition> for CurrentPosition {
    fn from(v: &kyogre_core::CurrentPosition) -> Self {
        let kyogre_core::CurrentPosition {
            vessel_id,
            latitude,
            longitude,
            timestamp,
            course_over_ground,
            speed,
            navigational_status,
            rate_of_turn,
            true_heading,
            distance_to_shore,
            position_type,
        } = v;

        Self {
            fiskeridir_vessel_id: *vessel_id,
            latitude: *latitude,
            longitude: *longitude,
            timestamp: *timestamp,
            course_over_ground: *course_over_ground,
            speed: *speed,
            navigation_status_id: *navigational_status,
            rate_of_turn: *rate_of_turn,
            true_heading: *true_heading,
            distance_to_shore: *distance_to_shore,
            position_type_id: *position_type,
        }
    }
}
