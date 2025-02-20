use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use serde::Deserialize;

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, strum::EnumIter)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum EarliestVmsUsedBy {
    TripsState = 1,
    CurrentTripPositionsProcessor = 2,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VmsPosition {
    pub call_sign: CallSign,
    pub course: Option<u32>,
    pub latitude: f64,
    pub longitude: f64,
    pub registration_id: Option<String>,
    pub speed: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub vessel_length: f64,
    pub vessel_name: String,
    pub vessel_type: String,
    pub distance_to_shore: f64,
}

impl From<EarliestVmsUsedBy> for i32 {
    fn from(value: EarliestVmsUsedBy) -> Self {
        value as i32
    }
}
