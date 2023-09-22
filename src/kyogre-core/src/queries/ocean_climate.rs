use chrono::{DateTime, Utc};

use crate::WeatherLocationId;

#[derive(Default, Debug, Clone)]
pub struct OceanClimateQuery {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub depths: Option<Vec<i32>>,
    pub weather_location_ids: Option<Vec<WeatherLocationId>>,
}
