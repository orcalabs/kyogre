use bigdecimal::FromPrimitive;
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use kyogre_core::NavigationStatus;

use crate::error::{FromBigDecimalError, NavigationStatusError, PostgresError};

#[derive(Debug, Clone)]
pub struct AisPosition {
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<BigDecimal>,
    pub navigational_status: i32,
    pub rate_of_turn: Option<BigDecimal>,
    pub speed_over_ground: Option<BigDecimal>,
    pub true_heading: Option<i32>,
}

impl TryFrom<AisPosition> for kyogre_core::AisPosition {
    type Error = Report<PostgresError>;

    fn try_from(value: AisPosition) -> Result<Self, Self::Error> {
        Ok(kyogre_core::AisPosition {
            latitude: value
                .latitude
                .to_f64()
                .ok_or(FromBigDecimalError(value.latitude))
                .into_report()
                .change_context(PostgresError::DataConversion)?,
            longitude: value
                .longitude
                .to_f64()
                .ok_or(FromBigDecimalError(value.longitude))
                .into_report()
                .change_context(PostgresError::DataConversion)?,
            mmsi: value.mmsi,
            msgtime: value.msgtime,
            course_over_ground: value
                .course_over_ground
                .map(|v| {
                    v.to_f64()
                        .ok_or(FromBigDecimalError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            navigational_status: NavigationStatus::from_i32(value.navigational_status)
                .ok_or(NavigationStatusError(value.navigational_status))
                .into_report()
                .change_context(PostgresError::DataConversion)?,
            rate_of_turn: value
                .rate_of_turn
                .map(|v| {
                    v.to_f64()
                        .ok_or(FromBigDecimalError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            speed_over_ground: value
                .speed_over_ground
                .map(|v| {
                    v.to_f64()
                        .ok_or(FromBigDecimalError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            true_heading: value.true_heading,
        })
    }
}