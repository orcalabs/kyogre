use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::NavigationStatus;

#[derive(Deserialize, Debug, Clone)]
pub struct AisVmsPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub speed: Option<f64>,
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: Option<f64>,
}
