use kyogre_core::CatchLocationId;

mod fishing_spot_predictor;
mod fishing_weight_predictor;

pub use fishing_spot_predictor::*;
pub use fishing_weight_predictor::*;

pub static HAUL_LIMIT_PER_SPECIES: u32 = 100;

#[derive(Debug, Hash, Eq, PartialEq)]
struct CatchLocationWeatherKey {
    pub week: u32,
    pub year: u32,
    pub catch_location_id: CatchLocationId,
}