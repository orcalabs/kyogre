use chrono::{DateTime, Utc};

use crate::FiskeridirVesselId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Departure {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub timestamp: DateTime<Utc>,
    pub port_id: Option<String>,
    pub message_number: i32,
}
