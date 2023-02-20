use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    pub vessel_id: i64,
    pub timestamp: DateTime<Utc>,
    pub port_code: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ArrivalFilter {
    WithLandingFacility,
    All,
}
