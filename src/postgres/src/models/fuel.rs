use chrono::{DateTime, Utc};
use kyogre_core::{FiskeridirVesselId, live_fuel_year_day_hour};
use unnest_insert::UnnestInsert;

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
    #[unnest_insert(update = "fuel_liter = live_fuel.fuel_liter + excluded.fuel_liter")]
    pub fuel_liter: f64,
}

impl UpsertNewLiveFuel {
    pub fn from_core(
        fiskeridir_vessel_id: FiskeridirVesselId,
        core: &kyogre_core::NewLiveFuel,
    ) -> Self {
        let &kyogre_core::NewLiveFuel {
            latest_position_timestamp,
            fuel_liter,
        } = core;
        let (year, day, hour) = live_fuel_year_day_hour(latest_position_timestamp);
        Self {
            year,
            day: day as i32,
            hour: hour as i32,
            fiskeridir_vessel_id,
            latest_position_timestamp: core.latest_position_timestamp,
            fuel_liter,
        }
    }
}
