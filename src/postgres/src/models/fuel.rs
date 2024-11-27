use crate::queries::type_to_i64;
use chrono::{DateTime, NaiveDate, Utc};
use kyogre_core::{BarentswatchUserId, FiskeridirVesselId, PositionType, TripId};
use unnest_insert::{UnnestDelete, UnnestInsert, UnnestUpdate};

#[derive(Debug, Clone, UnnestUpdate)]
#[unnest_update(table_name = "trip_positions")]
pub struct UpdateTripPositionFuel {
    #[unnest_update(id, sql_type = "BIGINT")]
    pub trip_id: TripId,
    #[unnest_update(id)]
    pub timestamp: DateTime<Utc>,
    #[unnest_update(id, sql_type = "INT")]
    pub position_type_id: PositionType,
    pub trip_cumulative_fuel_consumption: f64,
}

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

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "fuel_estimates", conflict = "fiskeridir_vessel_id, date")]
pub struct UpsertFuelEstimation {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub date: NaiveDate,
    #[unnest_insert(update)]
    pub estimate: f64,
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

impl From<&kyogre_core::UpdateTripPositionFuel> for UpdateTripPositionFuel {
    fn from(v: &kyogre_core::UpdateTripPositionFuel) -> Self {
        Self {
            trip_id: v.trip_id,
            timestamp: v.timestamp,
            position_type_id: v.position_type_id,
            trip_cumulative_fuel_consumption: v.trip_cumulative_fuel_consumption,
        }
    }
}

impl From<&kyogre_core::NewFuelDayEstimate> for UpsertFuelEstimation {
    fn from(v: &kyogre_core::NewFuelDayEstimate) -> Self {
        Self {
            fiskeridir_vessel_id: v.vessel_id,
            date: v.date,
            estimate: v.estimate,
        }
    }
}
