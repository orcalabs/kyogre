use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    pub vessel_id: i64,
    pub timestamp: DateTime<Utc>,
    pub port_code: Option<String>,
}
