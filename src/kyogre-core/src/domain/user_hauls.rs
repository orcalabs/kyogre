use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use fiskeridir_rs::Gear;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

use crate::Mmsi;

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

#[derive(Debug, Clone)]
pub struct UserHaulDistanceUpdate {
    pub id: UserHaulId,
    pub distance_meters: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct UserHaulWithoutDistance {
    pub id: UserHaulId,
    pub mmsi: Mmsi,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
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

impl PartialEq<UpdateUserHaul> for UserHaul {
    fn eq(&self, other: &UpdateUserHaul) -> bool {
        let UpdateUserHaul {
            start_ts,
            end_ts,
            start_fuel_liter,
            end_fuel_liter,
            total_living_weight_kg,
            config,
            gear,
        } = other;

        start_ts.timestamp_millis() == self.start_ts.timestamp_millis()
            && end_ts.timestamp_millis() == self.end_ts.timestamp_millis()
            && *start_fuel_liter == self.start_fuel_liter
            && *end_fuel_liter == self.end_fuel_liter
            && *total_living_weight_kg == self.total_living_weight_kg
            && *config == self.config
            && *gear == self.gear
    }
}

impl PartialEq<UserHaul> for UpdateUserHaul {
    fn eq(&self, other: &UserHaul) -> bool {
        other.eq(self)
    }
}

impl PartialEq<HaulEnd> for UserHaul {
    fn eq(&self, other: &HaulEnd) -> bool {
        let HaulEnd {
            fuel_liter_end,
            total_living_weight_kg,
        } = other;

        *fuel_liter_end == self.end_fuel_liter
            && *total_living_weight_kg == self.total_living_weight_kg
    }
}

impl PartialEq<UserHaul> for HaulEnd {
    fn eq(&self, other: &UserHaul) -> bool {
        other.eq(self)
    }
}

impl PartialEq<HaulStart> for UserHaul {
    fn eq(&self, other: &HaulStart) -> bool {
        let HaulStart {
            fuel_liter_start,
            config,
            gear,
        } = other;

        *fuel_liter_start == self.start_fuel_liter && *config == self.config && *gear == self.gear
    }
}

impl PartialEq<UserHaul> for HaulStart {
    fn eq(&self, other: &UserHaul) -> bool {
        other.eq(self)
    }
}

impl PartialEq<StartedUserHaul> for HaulStart {
    fn eq(&self, other: &StartedUserHaul) -> bool {
        let StartedUserHaul {
            id: _,
            start_ts: _,
            start_fuel_liter,
            config,
            gear,
        } = other;

        *start_fuel_liter == self.fuel_liter_start && *config == self.config && *gear == self.gear
    }
}

impl PartialEq<HaulStart> for StartedUserHaul {
    fn eq(&self, other: &HaulStart) -> bool {
        other.eq(self)
    }
}

#[cfg(feature = "test")]
mod test {
    use super::*;
    use crate::UserHaulId;
    use chrono::Duration;
    use serde_json::json;

    impl UserHaulId {
        pub fn test_new() -> Self {
            Self(rand::random())
        }
    }

    impl UpdateUserHaul {
        pub fn test_default() -> Self {
            let start_ts = Utc::now();
            Self {
                start_ts,
                end_ts: start_ts + Duration::seconds(10),
                start_fuel_liter: 20,
                end_fuel_liter: 10,
                total_living_weight_kg: Some(42.0),
                config: json!("kule: 28"),
                gear: Gear::TripleTrawl,
            }
        }
    }

    impl HaulStart {
        pub fn test_default() -> Self {
            use serde_json::json;

            Self {
                fuel_liter_start: 1000,
                config: json!("bobbins: 23"),
                gear: Gear::TripleTrawl,
            }
        }
    }

    impl HaulEnd {
        pub fn test_default() -> Self {
            Self {
                fuel_liter_end: 500,
                total_living_weight_kg: Some(20.0),
            }
        }
    }
}
