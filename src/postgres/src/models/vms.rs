use crate::{
    error::PostgresError,
    queries::{decimal_to_float, opt_decimal_to_float},
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use fiskeridir_rs::CallSign;
use serde::Deserialize;

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
}

impl TryFrom<VmsPosition> for kyogre_core::VmsPosition {
    type Error = Report<PostgresError>;

    fn try_from(value: VmsPosition) -> Result<Self, Self::Error> {
        Ok(kyogre_core::VmsPosition {
            latitude: decimal_to_float(value.latitude)
                .change_context(PostgresError::DataConversion)?,
            longitude: decimal_to_float(value.longitude)
                .change_context(PostgresError::DataConversion)?,
            course: value.course.map(|c| c as u32),
            speed: opt_decimal_to_float(value.speed)
                .change_context(PostgresError::DataConversion)?,
            call_sign: CallSign::try_from(value.call_sign)
                .change_context(PostgresError::DataConversion)?,
            registration_id: value.registration_id,
            timestamp: value.timestamp,
            vessel_length: decimal_to_float(value.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_name: value.vessel_name,
            vessel_type: value.vessel_type,
        })
    }
}
