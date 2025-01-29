use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use serde::{Deserialize, Serialize};

use crate::BarentswatchUserId;

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct FuelMeasurementId(i64);

#[derive(Debug, Clone)]
pub struct FuelMeasurement {
    pub id: FuelMeasurementId,
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone)]
pub struct CreateFuelMeasurement {
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone)]
pub struct DeleteFuelMeasurement {
    pub id: FuelMeasurementId,
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: CallSign,
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
