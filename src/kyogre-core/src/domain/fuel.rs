use crate::BarentswatchUserId;
use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use fiskeridir_rs::{CallSign, FiskeridirVesselId};
use serde::{Deserialize, Serialize};

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
#[serde(rename_all = "camelCase")]
pub struct FuelEntry {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub estimated_fuel: f64,
}

#[derive(Debug, Clone)]
pub struct NewLiveFuel {
    pub latest_position_timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct LiveFuelEntry {
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct LiveFuel {
    pub total_fuel: f64,
    pub entries: Vec<LiveFuelEntry>,
}

pub fn live_fuel_year_day_hour(ts: DateTime<Utc>) -> (i32, u32, u32) {
    (ts.year(), ts.ordinal(), ts.hour())
}
