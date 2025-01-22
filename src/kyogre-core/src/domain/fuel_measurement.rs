use chrono::{DateTime, Utc};
use fiskeridir_rs::FiskeridirVesselId;
use serde::{Deserialize, Serialize};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

use super::DateRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct FuelMeasurementId(i64);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[serde(rename_all = "camelCase")]
pub struct FuelMeasurement {
    pub id: FuelMeasurementId,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[serde(rename_all = "camelCase")]
pub struct CreateFuelMeasurement {
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

//#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
//#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
//#[serde(rename_all = "camelCase")]
//pub struct UpdateFuelMeasurement {
//    pub id: FuelMeasurementId,
//    pub timestamp: DateTime<Utc>,
//    pub fuel: f64,
//}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[serde(rename_all = "camelCase")]
pub struct DeleteFuelMeasurement {
    pub id: FuelMeasurementId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuelMeasurementRange {
    pub fuel_used: f64,
    pub fuel_range: DateRange,
    pub pre_estimate_ts: DateTime<Utc>,
    pub pre_estimate_value: Option<f64>,
    pub post_estimate_ts: DateTime<Utc>,
    pub post_estimate_value: Option<f64>,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateFuelMeasurementRange {
    pub fuel_range: DateRange,
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub pre_fuel: f64,
    pub post_fuel: f64,
}

impl From<FuelMeasurementId> for i64 {
    fn from(value: FuelMeasurementId) -> Self {
        value.0
    }
}

#[cfg(feature = "test")]
mod test {
    use super::*;

    impl FuelMeasurementId {
        pub fn test_new(value: i64) -> Self {
            Self(value)
        }
    }
}
