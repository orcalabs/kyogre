use chrono::{DateTime, Utc};
use kyogre_core::BarentswatchUserId;
use unnest_insert::{UnnestDelete, UnnestInsert, UnnestUpdate};
use uuid::Uuid;

use crate::error::PostgresErrorWrapper;

#[derive(Debug, Clone, UnnestInsert, UnnestUpdate)]
#[unnest_insert(table_name = "fuel_measurements")]
#[unnest_update(table_name = "fuel_measurements")]
pub struct FuelMeasurement {
    #[unnest_update(id)]
    pub barentswatch_user_id: Uuid,
    #[unnest_update(id)]
    pub call_sign: String,
    #[unnest_update(id)]
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, UnnestDelete)]
#[unnest_delete(table_name = "fuel_measurements")]
pub struct DeleteFuelMeasurement {
    pub barentswatch_user_id: Uuid,
    pub call_sign: String,
    pub timestamp: DateTime<Utc>,
}

impl TryFrom<FuelMeasurement> for kyogre_core::FuelMeasurement {
    type Error = PostgresErrorWrapper;

    fn try_from(v: FuelMeasurement) -> Result<Self, Self::Error> {
        Ok(Self {
            barentswatch_user_id: BarentswatchUserId(v.barentswatch_user_id),
            call_sign: v.call_sign.try_into()?,
            timestamp: v.timestamp,
            fuel: v.fuel,
        })
    }
}

impl From<kyogre_core::FuelMeasurement> for FuelMeasurement {
    fn from(v: kyogre_core::FuelMeasurement) -> Self {
        Self {
            barentswatch_user_id: v.barentswatch_user_id.0,
            call_sign: v.call_sign.into_inner(),
            timestamp: v.timestamp,
            fuel: v.fuel,
        }
    }
}

impl From<kyogre_core::DeleteFuelMeasurement> for DeleteFuelMeasurement {
    fn from(v: kyogre_core::DeleteFuelMeasurement) -> Self {
        Self {
            barentswatch_user_id: v.barentswatch_user_id.0,
            call_sign: v.call_sign.into_inner(),
            timestamp: v.timestamp,
        }
    }
}
