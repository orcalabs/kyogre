use crate::error::{
    AisMessageError,
    ais_message_error::{InvalidEtaSnafu, ParseEtaSnafu},
};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{
    AisClass, Mmsi, NavigationStatus, NewAisPosition, NewAisStatic, distance_to_shore,
};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tracing::warn;

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
    type Error = AisMessageError;

    fn try_from(a: AisStatic) -> Result<Self, Self::Error> {
        let AisStatic {
            message_type,
            message_type_id,
            mmsi,
            msgtime,
            imo_number,
            call_sign,
            destination,
            eta,
            name,
            draught,
            ship_length,
            ship_width,
            ship_type,
            dimension_a,
            dimension_b,
            dimension_c,
            dimension_d,
            position_fixing_device_type,
            report_class,
        } = a;

        let eta = eta.map(|eta| parse_eta_value(&eta)).transpose();
        let eta = match eta {
            Ok(v) => v.flatten(),
            Err(e) => {
                warn!("invalid eta: {e:?}");
                None
            }
        };

        let call_sign = call_sign.and_then(|v| match CallSign::try_from(v) {
            Ok(v) => Some(v),
            Err(e) => {
                warn!("invalid call_sign: {e:?}");
                None
            }
        });

        Ok(NewAisStatic {
            message_type: message_type.map(kyogre_core::AisMessageType::from),
            message_type_id,
            mmsi,
            msgtime,
            imo_number,
            call_sign,
            destination,
            eta,
            name,
            draught,
            ship_length,
            ship_width,
            ship_type,
            dimension_a,
            dimension_b,
            dimension_c,
            dimension_d,
            position_fixing_device_type,
            report_class,
        })
    }
}

impl From<AisPosition> for Option<NewAisPosition> {
    fn from(a: AisPosition) -> Self {
        match (a.latitude, a.longitude) {
            (Some(latitude), Some(longitude)) => {
                let AisPosition {
                    message_type_id,
                    message_type,
                    mmsi,
                    msgtime,
                    altitude,
                    course_over_ground,
                    latitude: _,
                    longitude: _,
                    navigational_status,
                    ais_class,
                    rate_of_turn,
                    speed_over_ground,
                    true_heading,
                } = a;

                Some(NewAisPosition {
                    latitude,
                    longitude,
                    message_type_id,
                    message_type: message_type.map(kyogre_core::AisMessageType::from),
                    mmsi,
                    msgtime,
                    altitude,
                    course_over_ground,
                    navigational_status,
                    ais_class,
                    rate_of_turn,
                    speed_over_ground,
                    true_heading,
                    distance_to_shore: distance_to_shore(latitude, longitude),
                })
            }
            _ => None,
        }
    }
}

impl PartialEq<kyogre_core::AisPosition> for AisPosition {
    fn eq(&self, other: &kyogre_core::AisPosition) -> bool {
        let Self {
            message_type_id: _,
            message_type: _,
            mmsi,
            msgtime,
            altitude: _,
            course_over_ground,
            latitude,
            longitude,
            navigational_status,
            ais_class: _,
            rate_of_turn,
            speed_over_ground,
            true_heading,
        } = self;

        latitude.unwrap() as i32 == other.latitude as i32
            && longitude.unwrap() as i32 == other.longitude as i32
            && *mmsi == other.mmsi
            && msgtime.timestamp() == other.msgtime.timestamp()
            && course_over_ground.map(|v| v as i32) == other.course_over_ground.map(|v| v as i32)
            && *navigational_status == other.navigational_status.unwrap()
            && rate_of_turn.map(|v| v as i32) == other.rate_of_turn.map(|v| v as i32)
            && speed_over_ground.map(|v| v as i32) == other.speed_over_ground.map(|v| v as i32)
            && *true_heading == other.true_heading
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
        let Self {
            message_type: _,
            message_type_id,
            mmsi,
            msgtime,
            imo_number,
            call_sign,
            destination,
            eta,
            name,
            draught,
            ship_length,
            ship_width,
            ship_type,
            dimension_a,
            dimension_b,
            dimension_c,
            dimension_d,
            position_fixing_device_type,
            report_class,
        } = self;

        other.mmsi == *mmsi
            && other.message_timestamp.timestamp() == msgtime.timestamp()
            && other.imo_number == *imo_number
            && other.call_sign.as_ref().map(|c| c.as_ref()) == call_sign.as_deref()
            && other.name == *name
            && other.draught == *draught
            && other.ship_width == *ship_width
            && other.ship_length == *ship_length
            && other.eta.map(|t| t.with_year(1980).unwrap().timestamp())
                == eta.as_ref().map(|t| {
                    let t = parse_eta_value(t).unwrap().unwrap();
                    t.with_year(1980)
                        .unwrap()
                        .with_second(0)
                        .unwrap()
                        .with_nanosecond(0)
                        .unwrap()
                        .timestamp()
                })
            && other.destination == *destination
            && other.message_type_id == *message_type_id as i32
            && other.ship_type == *ship_type
            && other.dimension_a == *dimension_a
            && other.dimension_b == *dimension_b
            && other.dimension_c == *dimension_c
            && other.dimension_d == *dimension_d
            && other.position_fixing_device_type == *position_fixing_device_type
            && other.report_class == *report_class
    }
}

impl PartialEq<kyogre_core::AisVessel> for AisStatic {
    fn eq(&self, other: &kyogre_core::AisVessel) -> bool {
        let Self {
            message_type: _,
            message_type_id: _,
            mmsi,
            msgtime: _,
            imo_number: _,
            call_sign,
            destination: _,
            eta: _,
            name,
            draught: _,
            ship_length: _,
            ship_width: _,
            ship_type: _,
            dimension_a: _,
            dimension_b: _,
            dimension_c: _,
            dimension_d: _,
            position_fixing_device_type: _,
            report_class: _,
        } = self;

        other.mmsi == *mmsi
            && other.call_sign.as_ref().map(|c| c.as_ref()) == call_sign.as_deref()
            && other.name == *name
    }
}

impl PartialEq<AisStatic> for kyogre_core::AisVessel {
    fn eq(&self, other: &AisStatic) -> bool {
        other.eq(self)
    }
}

fn parse_eta_value(val: &str) -> Result<Option<DateTime<Utc>>, AisMessageError> {
    match val.len() {
        0 => Ok(None),
        8 => {
            let month = &val[0..=1]
                .parse::<u32>()
                .with_context(|_| ParseEtaSnafu { eta: val })?;
            let day = &val[2..=3]
                .parse::<u32>()
                .with_context(|_| ParseEtaSnafu { eta: val })?;
            let hour = &val[4..=5]
                .parse::<u32>()
                .with_context(|_| ParseEtaSnafu { eta: val })?;
            let minute = &val[6..=7]
                .parse::<u32>()
                .with_context(|_| ParseEtaSnafu { eta: val })?;

            // See https://gpsd.gitlab.io/gpsd/AIVDM.html#_type_5_static_and_voyage_related_data
            // for default values
            if *month == 0 || *day == 0 || *hour == 24 || *minute == 60 {
                return Ok(None);
            }

            let year = chrono::Utc::now().year();

            let time = NaiveTime::from_hms_opt(*hour, *minute, 0)
                .ok_or_else(|| InvalidEtaSnafu { eta: val }.build())?;
            let date = NaiveDate::from_ymd_opt(year, *month, *day)
                .ok_or_else(|| InvalidEtaSnafu { eta: val }.build())?;
            let dt = NaiveDateTime::new(date, time);

            Ok(Some(Utc.from_utc_datetime(&dt)))
        }
        _ => InvalidEtaSnafu { eta: val }.fail(),
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

#[cfg(feature = "test")]
mod test {
    use rand::{Rng, random};

    use super::*;

    impl AisPosition {
        pub fn test_default(mmsi: Option<Mmsi>) -> AisPosition {
            AisPosition {
                message_type_id: Some(1),
                message_type: Some(AisMessageType::Position),
                mmsi: mmsi.unwrap_or_else(|| Mmsi::test_new(random::<i32>())),
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
            let mmsi: i32 = rand::rng().random();
            AisStatic {
                message_type_id: 5,
                message_type: Some(AisMessageType::Static),
                mmsi: Mmsi::test_new(mmsi.abs()),
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
}
