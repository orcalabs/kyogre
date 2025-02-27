use std::ops::AddAssign;

use chrono::{DateTime, Datelike, NaiveDate, Timelike, Utc};
use fiskeridir_rs::FiskeridirVesselId;
use serde::{Deserialize, Serialize};

use super::{DailyFuelEstimationPosition, PositionType};

#[derive(Debug, Clone)]
pub struct NewFuelDayEstimate {
    pub vessel_id: FiskeridirVesselId,
    pub engine_version: u32,
    pub date: NaiveDate,
    pub estimate_liter: f64,
    pub num_ais_positions: u32,
    pub num_vms_positions: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ComputedFuelEstimation {
    pub fuel_liter: f64,
    pub num_ais_positions: u32,
    pub num_vms_positions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[serde(rename_all = "camelCase")]
pub struct FuelEntry {
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    #[serde(rename = "estimatedFuel")]
    pub estimated_fuel_liter: f64,
}

#[derive(Debug, Clone)]
pub struct NewLiveFuel {
    pub latest_position_timestamp: DateTime<Utc>,
    pub fuel_liter: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct LiveFuelEntry {
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "fuel")]
    pub fuel_liter: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct LiveFuel {
    #[serde(rename = "totalFuel")]
    pub total_fuel_liter: f64,
    pub entries: Vec<LiveFuelEntry>,
}

pub fn live_fuel_year_day_hour(ts: DateTime<Utc>) -> (i32, u32, u32) {
    (ts.year(), ts.ordinal(), ts.hour())
}

impl AddAssign<ComputedFuelEstimation> for NewFuelDayEstimate {
    fn add_assign(&mut self, rhs: ComputedFuelEstimation) {
        let ComputedFuelEstimation {
            fuel_liter,
            num_ais_positions,
            num_vms_positions,
        } = rhs;

        self.estimate_liter += fuel_liter;
        self.num_ais_positions += num_ais_positions;
        self.num_vms_positions += num_vms_positions;
    }
}

impl AddAssign<&DailyFuelEstimationPosition> for NewFuelDayEstimate {
    fn add_assign(&mut self, rhs: &DailyFuelEstimationPosition) {
        match rhs.position_type_id {
            PositionType::Ais => self.num_ais_positions += 1,
            PositionType::Vms => self.num_vms_positions += 1,
        }
    }
}
