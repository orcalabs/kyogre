use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct DataMessage {
    pub positions: Vec<AisPosition>,
    pub static_messages: Vec<AisStatic>,
}

/// Vessel related data that is emitted every 6th minute from vessels.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisStatic {
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    #[serde(rename = "messageType")]
    pub message_type: u32,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    #[serde(rename = "imoNumber")]
    pub imo_number: Option<i32>,
    #[serde(rename = "callSign")]
    pub call_sign: Option<String>,
    pub destination: Option<String>,
    pub eta: Option<String>,
    pub name: Option<String>,
    pub draught: Option<i32>,
    #[serde(rename = "shipLength")]
    pub ship_length: Option<i32>,
    #[serde(rename = "shipWidth")]
    pub ship_width: Option<i32>,
    #[serde(rename = "shipType")]
    pub ship_type: Option<i32>,
    #[serde(rename = "dimensionA")]
    pub dimension_a: Option<i32>,
    #[serde(rename = "dimensionB")]
    pub dimension_b: Option<i32>,
    #[serde(rename = "dimensionC")]
    pub dimension_c: Option<i32>,
    #[serde(rename = "dimensionD")]
    pub dimension_d: Option<i32>,
    #[serde(rename = "positionFixingDeviceType")]
    pub position_fixing_device_type: Option<i32>,
    #[serde(rename = "reportClass")]
    pub report_class: Option<String>,
}

/// Position data that is emitted every 6th second by vessels.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisPosition {
    #[serde(rename = "messageType")]
    pub message_type: Option<i32>,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub altitude: Option<i32>,
    #[serde(rename = "courseOverGround")]
    pub course_over_ground: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    #[serde(rename = "navigationalStatus")]
    pub navigational_status: i32,
    #[serde(rename = "aisClass")]
    pub ais_class: Option<String>,
    #[serde(rename = "rateOfTurn")]
    pub rate_of_turn: Option<f64>,
    #[serde(rename = "speedOverGround")]
    pub speed_over_ground: Option<f64>,
    #[serde(rename = "trueHeading")]
    pub true_heading: Option<i32>,
}
