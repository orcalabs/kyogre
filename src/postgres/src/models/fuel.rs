use chrono::{DateTime, Utc};
use kyogre_core::{live_fuel_year_day_hour, FiskeridirVesselId, PositionType, TripId};
use unnest_insert::{UnnestInsert, UnnestUpdate};

use crate::queries::type_to_i64;

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "live_fuel",
    conflict = "fiskeridir_vessel_id, year, day, hour",
    where_clause = "live_fuel.latest_position_timestamp < excluded.latest_position_timestamp "
)]
pub struct UpsertNewLiveFuel {
    pub year: i32,
    pub day: i32,
    pub hour: i32,
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    #[unnest_insert(update)]
    pub latest_position_timestamp: DateTime<Utc>,
    #[unnest_insert(update = "fuel = live_fuel.fuel + excluded.fuel")]
    pub fuel: f64,
}

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

impl UpsertNewLiveFuel {
    pub fn from_core(
        fiskeridir_vessel_id: FiskeridirVesselId,
        core: &kyogre_core::NewLiveFuel,
    ) -> Self {
        let &kyogre_core::NewLiveFuel {
            latest_position_timestamp,
            fuel,
        } = core;
        let (year, day, hour) = live_fuel_year_day_hour(latest_position_timestamp);
        Self {
            year,
            day: day as i32,
            hour: hour as i32,
            fiskeridir_vessel_id,
            latest_position_timestamp: core.latest_position_timestamp,
            fuel,
        }
    }
}
