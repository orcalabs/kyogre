use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use kyogre_core::{NavigationStatus, PositionType, TripPositionLayerId};

use crate::{
    error::{NavigationStatusError, PostgresError},
    queries::{decimal_to_float, opt_decimal_to_float},
};

#[derive(Debug, Clone)]
pub struct AisVmsPosition {
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<BigDecimal>,
    pub speed: Option<BigDecimal>,
    pub navigational_status: Option<i32>,
    pub rate_of_turn: Option<BigDecimal>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: BigDecimal,
    pub position_type_id: PositionType,
    pub pruned_by: Option<TripPositionLayerId>,
}

impl TryFrom<AisVmsPosition> for kyogre_core::AisVmsPosition {
    type Error = Report<PostgresError>;

    fn try_from(v: AisVmsPosition) -> Result<Self, Self::Error> {
        Ok(Self {
            latitude: decimal_to_float(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: decimal_to_float(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            timestamp: v.timestamp,
            course_over_ground: opt_decimal_to_float(v.course_over_ground)
                .change_context(PostgresError::DataConversion)?,
            speed: opt_decimal_to_float(v.speed).change_context(PostgresError::DataConversion)?,
            navigational_status: v
                .navigational_status
                .map(|v| {
                    NavigationStatus::from_i32(v)
                        .ok_or(NavigationStatusError(v))
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            rate_of_turn: opt_decimal_to_float(v.rate_of_turn)
                .change_context(PostgresError::DataConversion)?,
            true_heading: v.true_heading,
            distance_to_shore: decimal_to_float(v.distance_to_shore)
                .change_context(PostgresError::DataConversion)?,
            position_type: v.position_type_id,
            pruned_by: v.pruned_by,
        })
    }
}
