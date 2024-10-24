use chrono::{DateTime, Utc};
use kyogre_core::BarentswatchUserId;
use unnest_insert::{UnnestDelete, UnnestInsert, UnnestUpdate};

#[derive(Debug, Clone, UnnestInsert, UnnestUpdate)]
#[unnest_insert(table_name = "fuel_measurements")]
#[unnest_update(table_name = "fuel_measurements")]
pub struct UpsertFuelMeasurement<'a> {
    #[unnest_insert(sql_type = "UUID")]
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
    #[unnest_delete(sql_type = "UUID")]
    pub barentswatch_user_id: BarentswatchUserId,
    pub call_sign: &'a str,
    pub timestamp: DateTime<Utc>,
}

impl<'a> From<&'a kyogre_core::FuelMeasurement> for UpsertFuelMeasurement<'a> {
    fn from(v: &'a kyogre_core::FuelMeasurement) -> Self {
        Self {
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
            barentswatch_user_id: v.barentswatch_user_id,
            call_sign: v.call_sign.as_ref(),
            timestamp: v.timestamp,
        }
    }
}
