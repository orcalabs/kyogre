use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use kyogre_core::{BarentswatchUserId, FuelMeasurementId};
use unnest_insert::{UnnestDelete, UnnestInsert, UnnestUpdate};

use crate::queries::type_to_i64;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "fuel_measurements",
    returning = "fuel_measurements_id:FuelMeasurementId,barentswatch_user_id:BarentswatchUserId,call_sign:CallSign,timestamp,fuel"
)]
pub struct CreateFuelMeasurement<'a> {
    #[unnest_insert(sql_type = "UUID")]
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: &'a str,
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, UnnestUpdate)]
#[unnest_update(table_name = "fuel_measurements")]
pub struct UpdateFuelMeasurement<'a> {
    #[unnest_update(id, sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fuel_measurements_id: FuelMeasurementId,
    #[unnest_update(id, sql_type = "UUID")]
    pub barentswatch_user_id: BarentswatchUserId,
    #[unnest_update(id)]
    pub call_sign: &'a str,
    #[unnest_update(id)]
    pub timestamp: DateTime<Utc>,
    pub fuel: f64,
}

#[derive(Debug, Clone, UnnestDelete)]
#[unnest_delete(table_name = "fuel_measurements")]
pub struct DeleteFuelMeasurement<'a> {
    #[unnest_delete(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fuel_measurements_id: FuelMeasurementId,
    #[unnest_delete(sql_type = "UUID")]
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: &'a str,
}

impl<'a> From<&'a kyogre_core::CreateFuelMeasurement> for CreateFuelMeasurement<'a> {
    fn from(v: &'a kyogre_core::CreateFuelMeasurement) -> Self {
        Self {
            barentswatch_user_id: v.barentswatch_user_id,
            call_sign: v.call_sign.as_ref(),
            timestamp: v.timestamp,
            fuel: v.fuel,
        }
    }
}

impl<'a> From<&'a kyogre_core::FuelMeasurement> for UpdateFuelMeasurement<'a> {
    fn from(v: &'a kyogre_core::FuelMeasurement) -> Self {
        Self {
            fuel_measurements_id: v.id,
            barentswatch_user_id: v.barentswatch_user_id,
            call_sign: v.call_sign.as_ref(),
            timestamp: v.timestamp,
            fuel: v.fuel,
        }
    }
}

impl<'a> From<&'a kyogre_core::DeleteFuelMeasurement> for DeleteFuelMeasurement<'a> {
    fn from(v: &'a kyogre_core::DeleteFuelMeasurement) -> Self {
        Self {
            fuel_measurements_id: v.id,
            barentswatch_user_id: v.barentswatch_user_id,
            call_sign: v.call_sign.as_ref(),
        }
    }
}
