use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::CallSign;

use crate::BarentswatchUserId;

#[derive(Debug, Clone)]
pub struct FuelMeasurementsQuery {
    pub barentswatch_user_id: BarentswatchUserId,
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
