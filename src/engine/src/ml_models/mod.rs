use chrono::{DateTime, Datelike, Duration, Utc};

mod fishing_spot_predictor;
mod fishing_weight_predictor;

pub use fishing_spot_predictor::*;
pub use fishing_weight_predictor::*;

fn max_week(current_time: DateTime<Utc>) -> (u32, bool) {
    let current_week = current_time.iso_week().week();
    let current_year = current_time.year();

    if (current_week == 52 || current_week == 53)
        && (current_time + Duration::weeks(1)).year() != current_year
    {
        (current_week, true)
    } else {
        (current_week + 1, false)
    }
}
