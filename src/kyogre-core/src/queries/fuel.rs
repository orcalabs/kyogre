use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::CallSign;

use crate::OptionalDateTimeRange;

pub static DEFAULT_LIVE_FUEL_THRESHOLD: Duration = Duration::days(1);

#[derive(Debug, Clone)]
pub struct FuelMeasurementsQuery {
    pub call_sign: CallSign,
    pub range: OptionalDateTimeRange,
}

#[derive(Debug, Clone)]
pub struct FuelQuery {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub call_sign: CallSign,
}

#[derive(Debug, Clone)]
pub struct LiveFuelQuery {
    pub threshold: DateTime<Utc>,
    pub call_sign: CallSign,
}
