use chrono::{DateTime, Utc};

use crate::WeatherLocationId;

#[derive(Default, Debug, Clone)]
pub struct WeatherQuery {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub weather_location_ids: Option<Vec<WeatherLocationId>>,
}
