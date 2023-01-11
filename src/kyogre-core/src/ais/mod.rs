use chrono::{DateTime, TimeZone, Utc};
use num_derive::FromPrimitive;
use rand::random;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::CallSign;
#[derive(Debug, Clone, Default)]
pub struct DataMessage {
    pub positions: Vec<NewAisPosition>,
    pub static_messages: Vec<NewAisStatic>,
}

#[derive(Debug, Clone)]
pub struct NewAisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub message_type_id: Option<i32>,
    pub message_type: Option<AisMessageType>,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub altitude: Option<i32>,
    pub course_over_ground: Option<f64>,
    pub navigational_status: NavigationStatus,
    pub ais_class: Option<AisClass>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

#[derive(Debug, Clone)]
pub struct NewAisStatic {
    pub message_type: Option<AisMessageType>,
    pub message_type_id: u32,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub imo_number: Option<i32>,
    pub call_sign: Option<String>,
    pub destination: Option<String>,
    pub eta: Option<DateTime<Utc>>,
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
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

#[derive(Debug, Clone)]
pub struct AisVessel {
    pub mmsi: i32,
    pub imo_number: Option<i32>,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub ship_length: Option<i32>,
    pub ship_width: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub destination: Option<String>,
}

#[derive(Clone, Debug)]
pub enum AisMessageType {
    /// A message containing position data.
    Position,
    /// A message containing vessel related data.
    Static,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AisClass {
    A,
    B,
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

#[derive(Debug)]
pub struct AisVesselMigrate {
    pub mmsi: i32,
    pub progress: Option<DateTime<Utc>>,
}

impl NewAisStatic {
    pub fn test_default(mmsi: i32, call_sign: &str) -> NewAisStatic {
        NewAisStatic {
            mmsi,
            imo_number: Some(random()),
            call_sign: Some(call_sign.to_owned()),
            name: Some("test_vessel".to_string()),
            ship_length: Some(random()),
            ship_width: Some(random()),
            eta: Some(Utc.timestamp_opt(1000, 0).unwrap()),
            destination: Some("ramfjord camping".to_string()),
            message_type: Some(AisMessageType::Static),
            message_type_id: 18,
            msgtime: Utc.timestamp_opt(900, 0).unwrap(),
            draught: Some(random()),
            ship_type: Some(random()),
        }
    }
}

impl NewAisPosition {
    pub fn test_default(mmsi: i32, time: DateTime<Utc>) -> NewAisPosition {
        NewAisPosition {
            latitude: random(),
            longitude: random(),
            message_type_id: Some(19),
            message_type: Some(AisMessageType::Position),
            mmsi,
            msgtime: time,
            altitude: Some(random()),
            course_over_ground: Some(random()),
            navigational_status: NavigationStatus::UnderWayUsingEngine,
            ais_class: Some(AisClass::A),
            rate_of_turn: Some(random()),
            speed_over_ground: Some(random()),
            true_heading: Some(random()),
            distance_to_shore: random(),
        }
    }
}
