use chrono::{DateTime, Utc};

use crate::WeatherLocationId;

#[derive(Default, Debug, Clone)]
pub struct WeatherQuery {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub weather_location_ids: Option<Vec<WeatherLocationId>>,
}
