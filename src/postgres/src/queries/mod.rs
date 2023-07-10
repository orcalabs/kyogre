use std::ops::Bound;

use crate::error::{BigDecimalError, FromBigDecimalError};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use error_stack::{report, Result};

pub mod ais;
pub mod ais_vms;
pub mod catch_area;
pub mod catch_location;
pub mod delivery_point;
pub mod duckdb;
pub mod economic_zone;
pub mod ers_common;
pub mod ers_dca;
pub mod ers_dep;
pub mod ers_por;
pub mod ers_tra;
pub mod fishing_facility;
pub mod gear;
pub mod hash;
pub mod haul;
pub mod landing;
pub mod landing_entry;
pub mod norwegian_land;
pub mod port;
pub mod species;
pub mod trip;
pub mod user;
pub mod vessel;
pub mod vessel_benchmarks;
pub mod vessel_events;
pub mod vms;

pub(crate) fn float_to_decimal(value: f64) -> Result<BigDecimal, BigDecimalError> {
    BigDecimal::from_f64(value).ok_or_else(|| report!(BigDecimalError(value)))
}

pub(crate) fn opt_float_to_decimal(
    value: Option<f64>,
) -> Result<Option<BigDecimal>, BigDecimalError> {
    value
        .map(|v| BigDecimal::from_f64(v).ok_or_else(|| report!(BigDecimalError(v))))
        .transpose()
}

pub(crate) fn decimal_to_float(value: BigDecimal) -> Result<f64, FromBigDecimalError> {
    bigdecimal::ToPrimitive::to_f64(&value).ok_or_else(|| report!(FromBigDecimalError(value)))
}

pub(crate) fn opt_decimal_to_float(
    value: Option<BigDecimal>,
) -> Result<Option<f64>, FromBigDecimalError> {
    value
        .map(|v| bigdecimal::ToPrimitive::to_f64(&v).ok_or_else(|| report!(FromBigDecimalError(v))))
        .transpose()
}

pub(crate) fn bound_float_to_decimal(
    value: Bound<f64>,
) -> Result<Bound<BigDecimal>, BigDecimalError> {
    Ok(match value {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Excluded(v) => Bound::Excluded(float_to_decimal(v)?),
        Bound::Included(v) => Bound::Included(float_to_decimal(v)?),
    })
}

pub(crate) fn opt_timestamp_from_date_and_time(
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
) -> Option<DateTime<Utc>> {
    match (date, time) {
        (Some(date), Some(time)) => Some(DateTime::from_utc(date.and_time(time), Utc)),
        _ => None,
    }
}
