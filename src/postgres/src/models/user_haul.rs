use chrono::{DateTime, Utc};
use kyogre_core::UserHaulId;

#[derive(Debug, Clone)]
pub struct StartedUserHaul {
    pub id: UserHaulId,
    pub start_ts: DateTime<Utc>,
    pub start_fuel_liter: i32,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct UserHaul {
    pub id: UserHaulId,
    pub start_ts: DateTime<Utc>,
    pub end_ts: DateTime<Utc>,
    pub start_fuel_liter: i32,
    pub end_fuel_liter: i32,
    pub config: serde_json::Value,
}

impl From<UserHaul> for kyogre_core::UserHaul {
    fn from(value: UserHaul) -> Self {
        let UserHaul {
            id,
            start_ts,
            end_ts,
            start_fuel_liter,
            end_fuel_liter,
            config,
        } = value;
        Self {
            id,
            start_ts,
            end_ts,
            start_fuel_liter: start_fuel_liter as u32,
            end_fuel_liter: end_fuel_liter as u32,
            config,
        }
    }
}

impl From<StartedUserHaul> for kyogre_core::StartedUserHaul {
    fn from(value: StartedUserHaul) -> Self {
        let StartedUserHaul {
            id,
            start_ts,
            start_fuel_liter,
            config,
        } = value;
        Self {
            id,
            start_ts,
            start_fuel_liter: start_fuel_liter as u32,
            config,
        }
    }
}
