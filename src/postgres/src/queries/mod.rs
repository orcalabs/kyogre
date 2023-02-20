use crate::error::BigDecimalError;
use bigdecimal::{BigDecimal, FromPrimitive};
use error_stack::{report, Result};

pub mod ais;
pub mod catch_areas;
pub mod delivery_points;
pub mod hashes;
pub mod landing_entries;
pub mod landings;
pub mod norwegian_land;
pub mod specie;
pub mod vessels;

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
