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
