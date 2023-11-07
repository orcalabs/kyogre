mod spot;
mod spot_weather;

pub use spot::*;
pub use spot_weather::*;

static PYTHON_FISHING_SPOT_CODE: &str =
    include_str!("../../../../../scripts/python/fishing_predictor/fishing_spot_predictor.py");
