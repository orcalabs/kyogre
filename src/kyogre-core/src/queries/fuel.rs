use chrono::{DateTime, Duration, NaiveDate, Utc};
use fiskeridir_rs::CallSign;

pub static DEFAULT_LIVE_FUEL_THRESHOLD: Duration = Duration::days(1);

#[derive(Debug, Clone)]
pub struct FuelMeasurementsQuery {
    pub call_sign: CallSign,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
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
