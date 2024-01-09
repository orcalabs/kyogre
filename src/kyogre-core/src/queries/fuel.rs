use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;

use crate::BarentswatchUserId;

#[derive(Debug, Clone)]
pub struct FuelMeasurementsQuery {
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}
