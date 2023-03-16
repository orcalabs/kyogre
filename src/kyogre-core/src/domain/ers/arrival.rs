use chrono::{DateTime, Utc};

use crate::FiskeridirVesselId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub timestamp: DateTime<Utc>,
    pub port_id: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum ArrivalFilter {
    WithLandingFacility,
    All,
}
