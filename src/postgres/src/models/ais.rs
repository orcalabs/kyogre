use std::fmt;

use bigdecimal::FromPrimitive;
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use kyogre_core::{Mmsi, NavigationStatus};
use unnest_insert::UnnestInsert;

use crate::error::{FromBigDecimalError, NavigationStatusError, PostgresError};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ais_vessels", conflict = "mmsi")]
pub struct NewAisVessel {
    pub mmsi: i32,
    #[unnest_insert(update_coalesce)]
    pub imo_number: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub call_sign: Option<String>,
    #[unnest_insert(update_coalesce)]
    pub name: Option<String>,
    #[unnest_insert(update_coalesce)]
    pub ship_width: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub ship_length: Option<i32>,
    #[unnest_insert(update_coalesce)]
    pub ship_type: Option<i32>,
    #[unnest_insert(update)]
    pub eta: Option<DateTime<Utc>>,
    #[unnest_insert(update_coalesce)]
    pub draught: Option<i32>,
    #[unnest_insert(update)]
    pub destination: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AisVesselMigrationProgress {
    pub mmsi: i32,
    pub progress: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AisPosition {
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub mmsi: i32,
    pub msgtime: DateTime<Utc>,
    pub course_over_ground: Option<BigDecimal>,
    pub navigational_status: Option<i32>,
    pub rate_of_turn: Option<BigDecimal>,
    pub speed_over_ground: Option<BigDecimal>,
    pub true_heading: Option<i32>,
    pub distance_to_shore: BigDecimal,
}

#[derive(Clone, Copy)]
pub enum AisClass {
    A,
    B,
}

impl fmt::Display for AisClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AisClass::A => f.write_str("A"),
            AisClass::B => f.write_str("B"),
        }
    }
}

impl From<kyogre_core::NewAisStatic> for NewAisVessel {
    fn from(v: kyogre_core::NewAisStatic) -> Self {
        Self {
            mmsi: v.mmsi.0,
            imo_number: v.imo_number,
            call_sign: v.call_sign,
            name: v.name,
            ship_width: v.ship_width,
            ship_length: v.ship_length,
            ship_type: v.ship_type,
            eta: v.eta,
            draught: v.draught,
            destination: v.destination,
        }
    }
}

impl From<kyogre_core::AisClass> for AisClass {
    fn from(value: kyogre_core::AisClass) -> Self {
        match value {
            kyogre_core::AisClass::A => AisClass::A,
            kyogre_core::AisClass::B => AisClass::B,
        }
    }
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
            mmsi: Mmsi(value.mmsi),
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
            navigational_status: value
                .navigational_status
                .map(|v| {
                    NavigationStatus::from_i32(v)
                        .ok_or(NavigationStatusError(v))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
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
            distance_to_shore: value
                .distance_to_shore
                .to_f64()
                .ok_or(FromBigDecimalError(value.distance_to_shore))
                .into_report()
                .change_context(PostgresError::DataConversion)?,
        })
    }
}
