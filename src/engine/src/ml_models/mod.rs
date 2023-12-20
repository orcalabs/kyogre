use chrono::NaiveDate;
use kyogre_core::CatchLocationId;

mod fishing_spot_predictor;
mod fishing_weight_predictor;

pub use fishing_spot_predictor::*;
pub use fishing_weight_predictor::*;

#[derive(Debug, Hash, Eq, PartialEq)]
struct CatchLocationWeatherKey {
    pub date: NaiveDate,
    pub catch_location_id: CatchLocationId,
}

static SYNODIC_MONTH: f64 = 29.53058770576;
// We want this as const, using the opt version with an unwrap is not allowed in stable rust
// per now.
#[allow(warnings)]
static KNOWN_FULL_MOON: NaiveDate = NaiveDate::from_ymd(1990, 1, 26);

// Formula source: https://www.omnicalculator.com/everyday-life/moon-phase
pub fn lunar_value(date: NaiveDate) -> f64 {
    (date - KNOWN_FULL_MOON).num_days() as f64 % SYNODIC_MONTH
}
