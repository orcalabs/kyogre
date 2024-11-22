use crate::error::{Error, MissingValueSnafu};
use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::distance_to_shore;
use serde::Deserialize;
use unnest_insert::UnnestInsert;

#[derive(Deserialize, Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "vms_positions", conflict = "call_sign, timestamp")]
pub struct NewVmsPosition<'a> {
    pub call_sign: &'a str,
    #[unnest_insert(conflict = "COALESCE(NULLIF(course, 0), excluded.course)")]
    pub course: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub gross_tonnage: Option<i32>,
    #[unnest_insert(update)]
    pub latitude: f64,
    #[unnest_insert(update)]
    pub longitude: f64,
    pub message_id: i32,
    pub message_type: &'a str,
    pub message_type_code: &'a str,
    #[unnest_insert(update_coalesce)]
    pub registration_id: Option<&'a str>,
    #[unnest_insert(conflict = "COALESCE(NULLIF(speed, 0), excluded.speed)")]
    pub speed: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub vessel_length: f64,
    pub vessel_name: &'a str,
    pub vessel_type: &'a str,
    #[unnest_insert(update)]
    pub distance_to_shore: f64,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "earliest_vms_insertion",
    conflict = "call_sign",
    where_clause = "earliest_vms_insertion.timestamp > excluded.timestamp"
)]
pub struct EarliestVms<'a> {
    pub call_sign: &'a str,
    #[unnest_insert(update)]
    pub timestamp: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VmsPosition {
    pub call_sign: CallSign,
    pub course: Option<i32>,
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

impl<'a> TryFrom<&'a fiskeridir_rs::Vms> for NewVmsPosition<'a> {
    type Error = Error;

    fn try_from(v: &'a fiskeridir_rs::Vms) -> Result<Self, Self::Error> {
        let latitude = v.latitude.ok_or_else(|| MissingValueSnafu.build())?;
        let longitude = v.longitude.ok_or_else(|| MissingValueSnafu.build())?;

        Ok(Self {
            call_sign: v.call_sign.as_ref(),
            course: v.course.map(|c| c as i32),
            gross_tonnage: v.gross_tonnage.map(|c| c as i32),
            latitude,
            longitude,
            message_id: v.message_id as i32,
            message_type: v.message_type.as_ref(),
            message_type_code: v.message_type_code.as_ref(),
            registration_id: v.registration_id.as_deref(),
            speed: v.speed,
            timestamp: v.timestamp,
            vessel_length: v.vessel_length,
            vessel_name: v.vessel_name.as_ref(),
            vessel_type: v.vessel_type.as_ref(),
            distance_to_shore: distance_to_shore(latitude, longitude),
        })
    }
}

impl From<VmsPosition> for kyogre_core::VmsPosition {
    fn from(value: VmsPosition) -> Self {
        let VmsPosition {
            call_sign,
            course,
            latitude,
            longitude,
            registration_id,
            speed,
            timestamp,
            vessel_length,
            vessel_name,
            vessel_type,
            distance_to_shore,
        } = value;

        Self {
            latitude,
            longitude,
            course: course.map(|c| c as u32),
            speed,
            call_sign,
            registration_id,
            timestamp,
            vessel_length,
            vessel_name,
            vessel_type,
            distance_to_shore,
        }
    }
}
