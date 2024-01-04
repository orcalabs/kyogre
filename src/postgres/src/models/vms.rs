use crate::{
    error::{PostgresError, PostgresErrorWrapper},
    queries::{decimal_to_float, float_to_decimal, opt_decimal_to_float, opt_float_to_decimal},
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::report;
use fiskeridir_rs::CallSign;
use kyogre_core::distance_to_shore;
use serde::Deserialize;
use unnest_insert::UnnestInsert;

#[derive(Deserialize, Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "vms_positions", conflict = "call_sign, timestamp")]
pub struct NewVmsPosition {
    pub call_sign: String,
    #[unnest_insert(conflict = "COALESCE(NULLIF(course, 0), excluded.course)")]
    pub course: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub gross_tonnage: Option<i32>,
    #[unnest_insert(update)]
    pub latitude: BigDecimal,
    #[unnest_insert(update)]
    pub longitude: BigDecimal,
    pub message_id: i32,
    pub message_type: String,
    pub message_type_code: String,
    #[unnest_insert(update_coalesce)]
    pub registration_id: Option<String>,
    #[unnest_insert(conflict = "COALESCE(NULLIF(speed, 0), excluded.speed)")]
    pub speed: Option<BigDecimal>,
    pub timestamp: DateTime<Utc>,
    pub vessel_length: BigDecimal,
    pub vessel_name: String,
    pub vessel_type: String,
    #[unnest_insert(update)]
    pub distance_to_shore: f64,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "earliest_vms_insertion",
    conflict = "call_sign",
    where_clause = "earliest_vms_insertion.timestamp > excluded.timestamp"
)]
pub struct EarliestVms {
    pub call_sign: String,
    #[unnest_insert(update)]
    pub timestamp: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VmsPosition {
    pub call_sign: String,
    pub course: Option<i32>,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub registration_id: Option<String>,
    pub speed: Option<BigDecimal>,
    pub timestamp: DateTime<Utc>,
    pub vessel_length: BigDecimal,
    pub vessel_name: String,
    pub vessel_type: String,
    pub distance_to_shore: f64,
}

impl TryFrom<fiskeridir_rs::Vms> for NewVmsPosition {
    type Error = PostgresErrorWrapper;

    fn try_from(v: fiskeridir_rs::Vms) -> Result<Self, Self::Error> {
        let latitude = v
            .latitude
            .ok_or_else(|| report!(PostgresError::DataConversion))?;
        let longitude = v
            .longitude
            .ok_or_else(|| report!(PostgresError::DataConversion))?;

        Ok(Self {
            call_sign: v.call_sign.into_inner(),
            course: v.course.map(|c| c as i32),
            gross_tonnage: v.gross_tonnage.map(|c| c as i32),
            latitude: float_to_decimal(latitude)?,
            longitude: float_to_decimal(longitude)?,
            message_id: v.message_id as i32,
            message_type: v.message_type,
            message_type_code: v.message_type_code,
            registration_id: v.registration_id,
            speed: opt_float_to_decimal(v.speed)?,
            timestamp: v.timestamp,
            vessel_length: float_to_decimal(v.vessel_length)?,
            vessel_name: v.vessel_name,
            vessel_type: v.vessel_type,
            distance_to_shore: distance_to_shore(latitude, longitude),
        })
    }
}

impl TryFrom<VmsPosition> for kyogre_core::VmsPosition {
    type Error = PostgresErrorWrapper;

    fn try_from(value: VmsPosition) -> Result<Self, Self::Error> {
        Ok(kyogre_core::VmsPosition {
            latitude: decimal_to_float(value.latitude)?,
            longitude: decimal_to_float(value.longitude)?,
            course: value.course.map(|c| c as u32),
            speed: opt_decimal_to_float(value.speed)?,
            call_sign: CallSign::try_from(value.call_sign)?,
            registration_id: value.registration_id,
            timestamp: value.timestamp,
            vessel_length: decimal_to_float(value.vessel_length)?,
            vessel_name: value.vessel_name,
            vessel_type: value.vessel_type,
            distance_to_shore: value.distance_to_shore,
        })
    }
}
