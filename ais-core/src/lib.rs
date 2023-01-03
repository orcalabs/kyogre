use chrono::{DateTime, Utc};
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

mod call_sign;

pub use call_sign::CallSign;

#[derive(Debug, Clone, Default)]
pub struct DataMessage {
    pub positions: Vec<NewAisPosition>,
    pub static_messages: Vec<NewAisStatic>,
}

#[derive(Debug, Clone)]
pub struct NewAisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub message_type: Option<i32>,
    pub type_name: Option<String>,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub altitude: Option<i32>,
    pub course_over_ground: Option<f64>,
    pub navigational_status: NavigationStatus,
    pub ais_class: Option<String>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct NewAisStatic {
    pub type_name: Option<String>,
    pub message_type: u32,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub imo_number: Option<i32>,
    pub call_sign: Option<String>,
    pub destination: Option<String>,
    pub eta: Option<String>,
    pub name: Option<String>,
    pub draught: Option<i32>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
    pub ship_type: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub navigational_status: NavigationStatus,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AisVessel {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    strum::Display,
)]
#[repr(u8)]
pub enum NavigationStatus {
    UnderWayUsingEngine = 0,
    AtAnchor = 1,
    NotUnderCommand = 2,
    RestrictedManoeuverability = 3,
    ConstrainedByDraught = 4,
    Moored = 5,
    Aground = 6,
    EngagedInFishing = 7,
    UnderWaySailing = 8,
    Reserved9 = 9,
    Reserved10 = 10,
    Reserved11 = 11,
    Reserved12 = 12,
    Reserved13 = 13,
    AisSartIsActive = 14,
    NotDefined = 15,
}
