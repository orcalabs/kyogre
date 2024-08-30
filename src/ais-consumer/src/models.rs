use error_stack::{bail, Report, Result, ResultExt};

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{
    distance_to_shore, AisClass, Mmsi, NavigationStatus, NewAisPosition, NewAisStatic,
};
use rand::{random, Rng};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::AisMessageError;

/// Vessel related data that is emitted every 6th minute from vessels.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AisStatic {
    #[serde(rename = "type")]
    pub message_type: Option<AisMessageType>,
    #[serde(rename = "messageType")]
    pub message_type_id: u32,
    pub mmsi: Mmsi,
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
    pub message_type_id: Option<i32>,
    #[serde(rename = "type")]
    pub message_type: Option<AisMessageType>,
    pub mmsi: Mmsi,
    pub msgtime: DateTime<Utc>,
    pub altitude: Option<i32>,
    #[serde(rename = "courseOverGround")]
    pub course_over_ground: Option<f64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    #[serde(rename = "navigationalStatus")]
    pub navigational_status: NavigationStatus,
    #[serde(rename = "aisClass")]
    pub ais_class: Option<AisClass>,
    #[serde(rename = "rateOfTurn")]
    pub rate_of_turn: Option<f64>,
    #[serde(rename = "speedOverGround")]
    pub speed_over_ground: Option<f64>,
    #[serde(rename = "trueHeading")]
    pub true_heading: Option<i32>,
}

pub struct NewAisPositionWrapper(pub Option<NewAisPosition>);

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum AisMessageType {
    /// A message containing position data.
    Position,
    /// A message containing vessel related data.
    #[serde(rename = "Staticdata")]
    Static,
}

/// Convenience struct to deserialize the message type prior to attempting to deserialize the full
/// message.
#[derive(Deserialize)]
pub struct MessageType {
    /// What type of message this is.
    #[serde(rename = "type")]
    pub message_type: AisMessageType,
}

pub enum AisMessage {
    Static(AisStatic),
    Position(AisPosition),
}

impl TryFrom<AisStatic> for NewAisStatic {
    type Error = Report<AisMessageError>;

    fn try_from(a: AisStatic) -> std::result::Result<Self, Self::Error> {
        let eta = a.eta.map(|eta| parse_eta_value(&eta)).transpose();
        let eta: Result<Option<Option<DateTime<Utc>>>, AisMessageError> = match eta {
            Ok(v) => Ok(v),
            Err(e) => {
                warn!("{e:?}");
                Ok(None)
            }
        };

        let call_sign: Result<Option<Option<CallSign>>, Report<fiskeridir_rs::Error>> = a
            .call_sign
            .map(|v| match CallSign::try_from(v) {
                Ok(v) => Ok(Some(v)),
                Err(e) => {
                    warn!("invalid call_sign: {e:?}");
                    Ok(None)
                }
            })
            .transpose();

        Ok(NewAisStatic {
            message_type: a.message_type.map(kyogre_core::AisMessageType::from),
            message_type_id: a.message_type_id,
            mmsi: a.mmsi,
            msgtime: a.msgtime,
            imo_number: a.imo_number,
            call_sign: call_sign.unwrap().flatten(),
            destination: a.destination,
            eta: eta?.flatten(),
            name: a.name,
            draught: a.draught,
            ship_length: a.ship_length,
            ship_width: a.ship_width,
            ship_type: a.ship_type,
            dimension_a: a.dimension_a,
            dimension_b: a.dimension_b,
            dimension_c: a.dimension_c,
            dimension_d: a.dimension_d,
            position_fixing_device_type: a.position_fixing_device_type,
            report_class: a.report_class,
        })
    }
}

impl From<AisPosition> for NewAisPositionWrapper {
    fn from(a: AisPosition) -> Self {
        match (a.latitude, a.longitude) {
            (Some(latitude), Some(longitude)) => NewAisPositionWrapper(Some(NewAisPosition {
                latitude,
                longitude,
                message_type_id: a.message_type_id,
                message_type: a.message_type.map(kyogre_core::AisMessageType::from),
                mmsi: a.mmsi,
                msgtime: a.msgtime,
                altitude: a.altitude,
                course_over_ground: a.course_over_ground,
                navigational_status: a.navigational_status,
                ais_class: a.ais_class,
                rate_of_turn: a.rate_of_turn,
                speed_over_ground: a.speed_over_ground,
                true_heading: a.true_heading,
                distance_to_shore: distance_to_shore(latitude, longitude),
            })),
            _ => NewAisPositionWrapper(None),
        }
    }
}

impl AisPosition {
    pub fn test_default(mmsi: Option<Mmsi>) -> AisPosition {
        AisPosition {
            message_type_id: Some(1),
            message_type: Some(AisMessageType::Position),
            mmsi: mmsi.unwrap_or_else(|| Mmsi(random::<i32>())),
            msgtime: chrono::offset::Utc::now(),
            altitude: Some(5),
            course_over_ground: Some(123.32),
            latitude: Some(12.23),
            longitude: Some(74.4),
            navigational_status: NavigationStatus::UnderWayUsingEngine,
            ais_class: Some(AisClass::A),
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
            message_type_id: 5,
            message_type: Some(AisMessageType::Static),
            mmsi: Mmsi(mmsi.abs()),
            msgtime: chrono::offset::Utc::now(),
            imo_number: Some(123),
            call_sign: Some("LK45".to_string()),
            destination: Some("BERGEN".to_string()),
            eta: Some(create_eta_string_value(
                &Utc.timestamp_opt(1000, 0).unwrap(),
            )),
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

impl PartialEq<kyogre_core::AisPosition> for AisPosition {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        self.latitude.unwrap() as i32 == other.latitude as i32
            && self.longitude.unwrap() as i32 == other.longitude as i32
            && self.mmsi == other.mmsi
            && self.msgtime.timestamp() == other.msgtime.timestamp()
            && self.course_over_ground.map(|v| v as i32)
                == other.course_over_ground.map(|v| v as i32)
            && self.navigational_status == other.navigational_status.unwrap()
            && self.rate_of_turn.map(|v| v as i32) == other.rate_of_turn.map(|v| v as i32)
            && self.speed_over_ground.map(|v| v as i32) == other.speed_over_ground.map(|v| v as i32)
            && self.true_heading == other.true_heading
    }
}

impl PartialEq<AisPosition> for kyogre_core::AisPosition {
    fn eq(&self, other: &AisPosition) -> bool {
        other.eq(self)
    }
}

pub fn create_eta_string_value(timestamp: &DateTime<Utc>) -> String {
    format!(
        "{:02}{:02}{:02}{:02}",
        timestamp.month(),
        timestamp.day(),
        timestamp.hour(),
        timestamp.minute()
    )
}

impl PartialEq<kyogre_core::AisVesselHistoric> for AisStatic {
    fn eq(&self, other: &kyogre_core::AisVesselHistoric) -> bool {
        other.mmsi == self.mmsi
            && other.imo_number == self.imo_number
            && other.call_sign.as_ref().map(|c| c.as_ref()) == self.call_sign.as_deref()
            && other.name == self.name
            && other.ship_width == self.ship_width
            && other.ship_length == self.ship_length
            && other.eta.map(|t| t.with_year(1980).unwrap().timestamp())
                == self.eta.as_ref().map(|t| {
                    let t = parse_eta_value(t).unwrap().unwrap();
                    t.with_year(1980)
                        .unwrap()
                        .with_second(0)
                        .unwrap()
                        .with_nanosecond(0)
                        .unwrap()
                        .timestamp()
                })
            && other.destination == self.destination
            && other.message_type_id == (self.message_type_id as i32)
            && other.ship_type == self.ship_type
            && other.dimension_a == self.dimension_a
            && other.dimension_b == self.dimension_b
            && other.dimension_c == self.dimension_c
            && other.dimension_d == self.dimension_d
            && other.position_fixing_device_type == self.position_fixing_device_type
            && other.report_class == self.report_class
    }
}

impl PartialEq<kyogre_core::AisVessel> for AisStatic {
    fn eq(&self, other: &kyogre_core::AisVessel) -> bool {
        other.mmsi == self.mmsi
            && other.imo_number == self.imo_number
            && other.call_sign.as_ref().map(|c| c.as_ref()) == self.call_sign.as_deref()
            && other.name == self.name
            && other.ship_width == self.ship_width
            && other.ship_length == self.ship_length
            && other.eta.map(|t| t.with_year(1980).unwrap().timestamp())
                == self.eta.as_ref().map(|t| {
                    let t = parse_eta_value(t).unwrap().unwrap();
                    t.with_year(1980)
                        .unwrap()
                        .with_second(0)
                        .unwrap()
                        .with_nanosecond(0)
                        .unwrap()
                        .timestamp()
                })
            && other.destination == self.destination
    }
}

impl PartialEq<AisStatic> for kyogre_core::AisVessel {
    fn eq(&self, other: &AisStatic) -> bool {
        other.eq(self)
    }
}

fn parse_eta_value(val: &str) -> Result<Option<DateTime<Utc>>, AisMessageError> {
    if val.is_empty() {
        Ok(None)
    } else if val.len() != 8 {
        bail!(AisMessageError::InvalidEta(val.to_string()))
    } else {
        let month = &val[0..=1]
            .parse::<u32>()
            .change_context(AisMessageError::InvalidEta(val.to_string()))?;
        let day = &val[2..=3]
            .parse::<u32>()
            .change_context(AisMessageError::InvalidEta(val.to_string()))?;

        let hour = &val[4..=5]
            .parse::<u32>()
            .change_context(AisMessageError::InvalidEta(val.to_string()))?;
        let minute = &val[6..=7]
            .parse::<u32>()
            .change_context(AisMessageError::InvalidEta(val.to_string()))?;
        let year = chrono::Utc::now().year();

        // See https://gpsd.gitlab.io/gpsd/AIVDM.html#_type_5_static_and_voyage_related_data
        // for default values
        if *month == 0 || *day == 0 || *hour == 24 || *minute == 60 {
            return Ok(None);
        }

        let time = NaiveTime::from_hms_opt(*hour, *minute, 0)
            .ok_or_else(|| AisMessageError::InvalidEta(val.to_string()))?;
        let date = NaiveDate::from_ymd_opt(year, *month, *day)
            .ok_or_else(|| AisMessageError::InvalidEta(val.to_string()))?;
        let dt = NaiveDateTime::new(date, time);

        Ok(Some(Utc.from_utc_datetime(&dt)))
    }
}

impl From<AisMessageType> for kyogre_core::AisMessageType {
    fn from(value: AisMessageType) -> Self {
        match value {
            AisMessageType::Position => kyogre_core::AisMessageType::Position,
            AisMessageType::Static => kyogre_core::AisMessageType::Static,
        }
    }
}
