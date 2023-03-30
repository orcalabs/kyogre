use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use serde::Deserialize;

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
}
