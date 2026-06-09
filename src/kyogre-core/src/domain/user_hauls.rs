use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use fiskeridir_rs::Gear;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct UserHaulId(i64);

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct UserHaul {
    pub id: UserHaulId,
    #[serde_as(as = "DisplayFromStr")]
    pub gear: Gear,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub start_fuel_liter: u32,
    pub end_fuel_liter: u32,
    pub total_living_weight_kg: Option<f64>,
    pub config: serde_json::Value,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct UpdateUserHaul {
    #[serde_as(as = "DisplayFromStr")]
    pub gear: Gear,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub start_fuel_liter: u32,
    pub end_fuel_liter: u32,
    pub total_living_weight_kg: Option<f64>,
    pub config: serde_json::Value,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct StartedUserHaul {
    pub id: UserHaulId,
    #[serde_as(as = "DisplayFromStr")]
    pub gear: Gear,
    pub start_ts: DateTime<Utc>,
    pub start_fuel_liter: u32,
    pub config: serde_json::Value,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct HaulStart {
    #[serde_as(as = "DisplayFromStr")]
    pub gear: Gear,
    pub fuel_liter_start: u32,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct HaulEnd {
    pub fuel_liter_end: u32,
    pub total_living_weight_kg: Option<f64>,
}

impl Display for UserHaulId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
