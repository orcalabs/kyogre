use ais_core::{NavigationStatus, NewAisPosition, NewAisStatic};
use chrono::{DateTime, TimeZone, Utc};
use rand::{random, Rng};
use serde::{Deserialize, Serialize};

use crate::error::AisMessageError;

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
    pub eta: Option<DateTime<Utc>>,
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
    pub navigational_status: NavigationStatus,
    #[serde(rename = "aisClass")]
    pub ais_class: Option<String>,
    #[serde(rename = "rateOfTurn")]
    pub rate_of_turn: Option<f64>,
    #[serde(rename = "speedOverGround")]
    pub speed_over_ground: Option<f64>,
    #[serde(rename = "trueHeading")]
    pub true_heading: Option<i32>,
}

pub struct NewAisPositionWrapper(pub Option<NewAisPosition>);

/// The AIS message types we support.
pub enum SupportedMessageTypes {
    /// A message containing position data.
    Position,
    /// A message containing vessel related data.
    Static,
}

/// Convenience struct to deserialize the message type prior to attempting to deserialize the full
/// message.
#[derive(Deserialize)]
pub struct MessageType {
    /// What type of message this is.
    #[serde(rename = "messageType")]
    pub message_type: u32,
}

pub enum AisMessage {
    Static(AisStatic),
    Position(AisPosition),
}

impl TryFrom<u32> for SupportedMessageTypes {
    type Error = AisMessageError;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            1 | 2 | 3 | 27 => Ok(SupportedMessageTypes::Position),
            5 | 18 | 19 | 24 => Ok(SupportedMessageTypes::Static),
            _ => Err(AisMessageError::InvalidMessageType(value)),
        }
    }
}

impl From<AisStatic> for NewAisStatic {
    fn from(a: AisStatic) -> Self {
        NewAisStatic {
            type_name: a.type_name,
            message_type: a.message_type,
            mmsi: a.mmsi,
            msgtime: a.msgtime,
            imo_number: a.imo_number,
            call_sign: a.call_sign,
            destination: a.destination,
            eta: a.eta,
            name: a.name,
            draught: a.draught,
            ship_length: a.ship_length,
            ship_width: a.ship_width,
            ship_type: a.ship_type,
        }
    }
}

impl From<AisPosition> for NewAisPositionWrapper {
    fn from(a: AisPosition) -> Self {
        match (a.latitude, a.longitude) {
            (Some(latitude), Some(longitude)) => NewAisPositionWrapper(Some(NewAisPosition {
                latitude,
                longitude,
                message_type: a.message_type,
                type_name: a.type_name,
                mmsi: a.mmsi,
                msgtime: a.msgtime,
                altitude: a.altitude,
                course_over_ground: a.course_over_ground,
                navigational_status: a.navigational_status,
                ais_class: a.ais_class,
                rate_of_turn: a.rate_of_turn,
                speed_over_ground: a.speed_over_ground,
                true_heading: a.true_heading,
            })),
            _ => NewAisPositionWrapper(None),
        }
    }
}

impl AisPosition {
    pub fn test_default(mmsi: Option<i32>) -> AisPosition {
        AisPosition {
            message_type: Some(1),
            type_name: Some("test_ais_message".to_string()),
            mmsi: mmsi.unwrap_or_else(random::<i32>),
            msgtime: chrono::offset::Utc::now(),
            altitude: Some(5),
            course_over_ground: Some(123.32),
            latitude: Some(12.23),
            longitude: Some(74.4),
            navigational_status: NavigationStatus::UnderWayUsingEngine,
            ais_class: Some("AIS_CLASS".to_string()),
            rate_of_turn: Some(43.23),
            speed_over_ground: Some(8.4),
            true_heading: Some(320),
        }
    }
}

impl AisStatic {
    pub fn test_default() -> AisStatic {
        let mmsi: i32 = rand::thread_rng().gen();
        AisStatic {
            message_type: 5,
            type_name: Some("ais_message".to_string()),
            mmsi: mmsi.abs(),
            msgtime: chrono::offset::Utc::now(),
            imo_number: Some(123),
            call_sign: Some("LK45".to_string()),
            destination: Some("BERGEN".to_string()),
            eta: Some(Utc.timestamp_opt(1000, 0).unwrap()),
            name: Some("sjarken".to_string()),
            draught: Some(213),
            ship_length: Some(23),
            ship_width: Some(8),
            ship_type: Some(2),
            dimension_a: Some(1),
            dimension_b: Some(2),
            dimension_c: Some(3),
            dimension_d: Some(4),
            position_fixing_device_type: Some(2),
            report_class: Some("test_report_class".to_string()),
        }
    }
}

impl PartialEq<ais_core::AisPosition> for AisPosition {
    fn eq(&self, other: &ais_core::AisPosition) -> bool {
        self.latitude.unwrap() as i32 == other.latitude as i32
            && self.longitude.unwrap() as i32 == other.longitude as i32
            && self.mmsi == other.mmsi
            && self.msgtime.timestamp() == other.msgtime.timestamp()
            && self.course_over_ground.map(|v| v as i32)
                == other.course_over_ground.map(|v| v as i32)
            && self.navigational_status == other.navigational_status
            && self.rate_of_turn.map(|v| v as i32) == other.rate_of_turn.map(|v| v as i32)
            && self.speed_over_ground.map(|v| v as i32) == other.speed_over_ground.map(|v| v as i32)
            && self.true_heading == other.true_heading
    }
}

impl PartialEq<AisPosition> for ais_core::AisPosition {
    fn eq(&self, other: &AisPosition) -> bool {
        other.eq(self)
    }
}

impl PartialEq<ais_core::AisVessel> for AisStatic {
    fn eq(&self, other: &ais_core::AisVessel) -> bool {
        other.mmsi == self.mmsi
            && other.imo_number == self.imo_number
            && other.call_sign.as_ref().map(|c| c.as_ref()) == self.call_sign.as_deref()
            && other.name == self.name
            && other.ship_width == self.ship_width
            && other.ship_length == self.ship_length
            && other.eta.map(|t| t.timestamp()) == self.eta.map(|t| t.timestamp())
            && other.destination == self.destination
    }
}

impl PartialEq<AisStatic> for ais_core::AisVessel {
    fn eq(&self, other: &AisStatic) -> bool {
        other.eq(self)
    }
}
