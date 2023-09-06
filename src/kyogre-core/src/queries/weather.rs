use chrono::{DateTime, Utc};

#[derive(Default, Debug, Clone)]
pub struct WeatherQuery {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub weather_location_ids: Option<Vec<i32>>,
}
