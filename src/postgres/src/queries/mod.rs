use crate::error::BigDecimalError;
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
    BigDecimal::from_f64(value)
        .ok_or(report!(BigDecimalError(value)).attach_printable(format!("{value:?}")))
}

pub(crate) fn opt_float_to_decimal(
    value: Option<f64>,
) -> Result<Option<BigDecimal>, BigDecimalError> {
    value
        .map(|v| {
            BigDecimal::from_f64(v)
                .ok_or(report!(BigDecimalError(v)).attach_printable(format!("{value:?}")))
        })
        .transpose()
}
