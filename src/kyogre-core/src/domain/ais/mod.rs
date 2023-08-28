use chrono::{DateTime, TimeZone, Utc};
use fiskeridir_rs::CallSign;
use num_derive::FromPrimitive;
use rand::random;
use serde::{de::Visitor, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

pub const LEISURE_VESSEL_SHIP_TYPES: [i32; 2] = [36, 37];
pub const LEISURE_VESSEL_LENGTH_AIS_BOUNDARY: u32 = 45;
pub const PRIVATE_AIS_DATA_VESSEL_LENGTH_BOUNDARY: u32 = 15;

// What AIS user is allowed to read, AIS data of leisure vessels under 45 are implicitly
// denied for all permissions
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AisPermission {
    All,
    #[default]
    Above15m,
}

#[derive(Debug, Clone, Default)]
pub struct DataMessage {
    pub positions: Vec<NewAisPosition>,
    pub static_messages: Vec<NewAisStatic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Mmsi(pub i32);

#[derive(Debug, Clone)]
pub struct NewAisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub message_type_id: Option<i32>,
    pub message_type: Option<AisMessageType>,
    pub mmsi: Mmsi,
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
    pub mmsi: Mmsi,
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
    pub dimension_a: Option<i32>,
    pub dimension_b: Option<i32>,
    pub dimension_c: Option<i32>,
    pub dimension_d: Option<i32>,
    pub position_fixing_device_type: Option<i32>,
    pub report_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AisPosition {
    pub latitude: f64,
    pub longitude: f64,
    pub mmsi: Mmsi,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<f64>,
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<f64>,
    pub speed_over_ground: Option<f64>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: f64,
}

#[derive(Debug, Clone)]
pub struct AisVesselHistoric {
    pub mmsi: Mmsi,
    pub imo_number: Option<i32>,
    pub message_type_id: i32,
    pub message_timestamp: DateTime<Utc>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub ship_width: Option<i32>,
    pub ship_length: Option<i32>,
    pub ship_type: Option<i32>,
    pub eta: Option<DateTime<Utc>>,
    pub draught: Option<i32>,
    pub destination: Option<String>,
    pub dimension_a: Option<i32>,
    pub dimension_b: Option<i32>,
    pub dimension_c: Option<i32>,
    pub dimension_d: Option<i32>,
    pub position_fixing_device_type: Option<i32>,
    pub report_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AisVessel {
    pub mmsi: Mmsi,
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
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
    pub mmsi: Mmsi,
    pub progress: Option<DateTime<Utc>>,
}

impl NewAisStatic {
    pub fn test_default(mmsi: Mmsi, call_sign: &str) -> NewAisStatic {
        NewAisStatic {
            mmsi,
            imo_number: Some(10),
            call_sign: Some(call_sign.to_owned()),
            name: Some("test_vessel".to_string()),
            ship_length: Some(10),
            ship_width: Some(5),
            eta: Some(Utc.timestamp_opt(1000, 0).unwrap()),
            destination: Some("ramfjord camping".to_string()),
            message_type: Some(AisMessageType::Static),
            message_type_id: 18,
            msgtime: Utc.timestamp_opt(900, 0).unwrap(),
            draught: Some(50),
            ship_type: Some(1),
            dimension_a: Some(1),
            dimension_b: Some(1),
            dimension_c: Some(1),
            dimension_d: Some(1),
            position_fixing_device_type: Some(1),
            report_class: Some("A".to_string()),
        }
    }
}

impl NewAisPosition {
    pub fn test_default(mmsi: Mmsi, time: DateTime<Utc>) -> NewAisPosition {
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

impl<'de> Deserialize<'de> for Mmsi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MmsiVisitor;

        impl<'de> Visitor<'de> for MmsiVisitor {
            type Value = Mmsi;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Mmsi value")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Mmsi(v as i32))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Mmsi(v as i32))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Mmsi(v.parse().map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                })?))
            }
        }

        deserializer.deserialize_i64(MmsiVisitor)
    }
}
