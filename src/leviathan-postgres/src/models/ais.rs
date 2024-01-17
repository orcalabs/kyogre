use crate::error::{FromBigDecimalError, PostgresError};
use chrono::{DateTime, Utc};
use error_stack::{report, Report};
use kyogre_core::Mmsi;
use num_traits::{FromPrimitive, ToPrimitive};
use sqlx::types::BigDecimal;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AisPosition {
    pub mmsi: i32,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub time: DateTime<Utc>,
    /// Speed over ground in knots
    pub speed: Option<BigDecimal>,
    /// Course over ground (incl. drift, wind)
    pub course_over_ground: Option<BigDecimal>,
    /// True heading (map direction)
    pub heading_true: Option<BigDecimal>,
    /// Distance to nearest port in Norway in meters
    pub distance_to_port: BigDecimal,
    /// Distance to nearest shoreline in Norway in meters
    pub distance_to_shore: BigDecimal,
    /// Whether this position has a high accuracy (true = high (<= 10 m), false = low (> 10 m))
    pub high_position_accuracy: Option<bool>,
    pub navigation_status_id: Option<i32>,
    pub rate_of_turn: Option<BigDecimal>,
}

impl TryFrom<AisPosition> for kyogre_core::AisPosition {
    type Error = Report<PostgresError>;

    fn try_from(value: AisPosition) -> Result<Self, Self::Error> {
        Ok(kyogre_core::AisPosition {
            latitude: value.latitude.to_f64().ok_or_else(|| {
                report!(FromBigDecimalError(value.latitude))
                    .change_context(PostgresError::DataConversion)
            })?,
            longitude: value.longitude.to_f64().ok_or_else(|| {
                report!(FromBigDecimalError(value.longitude))
                    .change_context(PostgresError::DataConversion)
            })?,
            mmsi: Mmsi(value.mmsi),
            msgtime: value.time,
            course_over_ground: value
                .course_over_ground
                .map(|c| {
                    c.to_f64().ok_or_else(|| {
                        report!(FromBigDecimalError(c))
                            .change_context(PostgresError::DataConversion)
                    })
                })
                .transpose()?,
            navigational_status: value
                .navigation_status_id
                .map(|c| {
                    kyogre_core::NavigationStatus::from_i32(c)
                        .ok_or_else(|| report!(PostgresError::DataConversion))
                })
                .transpose()?,

            rate_of_turn: value
                .rate_of_turn
                .map(|c| {
                    c.to_f64().ok_or_else(|| {
                        report!(FromBigDecimalError(c))
                            .change_context(PostgresError::DataConversion)
                    })
                })
                .transpose()?,
            speed_over_ground: value
                .speed
                .map(|c| {
                    c.to_f64().ok_or_else(|| {
                        report!(FromBigDecimalError(c))
                            .change_context(PostgresError::DataConversion)
                    })
                })
                .transpose()?,
            true_heading: value
                .heading_true
                .map(|c| {
                    c.to_i32()
                        .ok_or_else(|| report!(PostgresError::DataConversion))
                })
                .transpose()?,
            distance_to_shore: value.distance_to_shore.to_f64().ok_or_else(|| {
                report!(FromBigDecimalError(value.distance_to_shore))
                    .change_context(PostgresError::DataConversion)
            })?,
        })
    }
}
