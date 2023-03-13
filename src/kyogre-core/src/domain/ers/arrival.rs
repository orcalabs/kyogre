use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    pub fiskeridir_vessel_id: i64,
    pub timestamp: DateTime<Utc>,
    pub port_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ArrivalFilter {
    WithLandingFacility,
    All,
}
