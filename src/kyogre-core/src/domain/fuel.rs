use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;

use crate::BarentswatchUserId;

#[derive(Debug, Clone)]
pub struct FuelMeasurement {
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone)]
pub struct DeleteFuelMeasurement {
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub timestamp: DateTime<Utc>,
}
