use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use kyogre_core::{NavigationStatus, PositionType, TripPositionLayerId};

use crate::{
    error::PostgresErrorWrapper,
    queries::{decimal_to_float, opt_decimal_to_float},
};

#[derive(Debug, Clone)]
pub struct AisVmsPosition {
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub timestamp: DateTime<Utc>,
    pub course_over_ground: Option<BigDecimal>,
    pub speed: Option<BigDecimal>,
    pub navigational_status: Option<NavigationStatus>,
    pub rate_of_turn: Option<BigDecimal>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: BigDecimal,
    pub position_type_id: PositionType,
    pub pruned_by: Option<TripPositionLayerId>,
}

impl TryFrom<AisVmsPosition> for kyogre_core::AisVmsPosition {
    type Error = PostgresErrorWrapper;

    fn try_from(v: AisVmsPosition) -> Result<Self, Self::Error> {
        Ok(Self {
            latitude: decimal_to_float(v.latitude)?,
            longitude: decimal_to_float(v.longitude)?,
            timestamp: v.timestamp,
            course_over_ground: opt_decimal_to_float(v.course_over_ground)?,
            speed: opt_decimal_to_float(v.speed)?,
            navigational_status: v.navigational_status,
            rate_of_turn: opt_decimal_to_float(v.rate_of_turn)?,
            true_heading: v.true_heading,
            distance_to_shore: decimal_to_float(v.distance_to_shore)?,
            position_type: v.position_type_id,
            pruned_by: v.pruned_by,
        })
    }
}
