use crate::error::{BigDecimalError, FromBigDecimalError};
use bigdecimal::{BigDecimal, FromPrimitive};
use error_stack::{report, Result};

pub mod ais;
pub mod catch_area;
pub mod delivery_point;
pub mod hash;
pub mod landing;
pub mod landing_entry;
pub mod norwegian_land;
pub mod species;
pub mod vessel;

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

pub(crate) fn opt_decimal_to_float(
    value: Option<BigDecimal>,
) -> Result<Option<f64>, FromBigDecimalError> {
    value
        .map(|v| bigdecimal::ToPrimitive::to_f64(&v).ok_or_else(|| report!(FromBigDecimalError(v))))
        .transpose()
}
