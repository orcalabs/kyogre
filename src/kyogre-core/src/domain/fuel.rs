use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::{CallSign, FiskeridirVesselId};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone)]
pub struct NewFuelDayEstimate {
    pub vessel_id: FiskeridirVesselId,
    pub date: NaiveDate,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct FuelEntry {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub estimated_fuel: f64,
}
