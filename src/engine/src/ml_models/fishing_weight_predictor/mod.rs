mod weight;
mod weight_weather;

static PYTHON_FISHING_WEIGHT_PREDICTOR_CODE: &str =
    include_str!("../../../../../scripts/python/fishing_predictor/fishing_weight_predictor.py");

pub use weight::*;
pub use weight_weather::*;
