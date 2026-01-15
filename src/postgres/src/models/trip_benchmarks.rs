use crate::queries::{type_to_i32, type_to_i64};
use kyogre_core::{ProcessingStatus, TripId};
use unnest_insert::UnnestUpdate;

#[derive(Debug, Clone, UnnestUpdate)]
#[unnest_update(
    table_name = "trips_detailed",
    where_clause = "t.benchmark_state_counter = q.benchmark_state_counter"
)]
pub struct TripBenchmarkOutput {
    #[unnest_update(id, sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub trip_id: TripId,
    pub benchmark_weight_per_hour: Option<f64>,
    pub benchmark_weight_per_distance: Option<f64>,
    pub benchmark_fuel_consumption_liter: Option<f64>,
    pub benchmark_weight_per_fuel_liter: Option<f64>,
    pub benchmark_catch_value_per_fuel_liter: Option<f64>,
    pub benchmark_eeoi: Option<f64>,
    #[unnest_update(sql_type = "INT", type_conversion = "type_to_i32")]
    pub benchmark_status: ProcessingStatus,
    // We do not update this (rows are only updated if this matches the value in the existing row),
    // but need it for the `where_clause`
    pub benchmark_state_counter: i32,
}

impl From<&kyogre_core::TripBenchmarkOutput> for TripBenchmarkOutput {
    fn from(v: &kyogre_core::TripBenchmarkOutput) -> Self {
        let kyogre_core::TripBenchmarkOutput {
            trip_id,
            weight_per_hour,
            weight_per_distance,
            fuel_consumption_liter,
            weight_per_fuel_liter,
            catch_value_per_fuel_liter,
            eeoi,
            status,
            benchmark_state_counter,
        } = v;

        Self {
            trip_id: *trip_id,
            benchmark_weight_per_hour: *weight_per_hour,
            benchmark_weight_per_distance: *weight_per_distance,
            benchmark_fuel_consumption_liter: *fuel_consumption_liter,
            benchmark_weight_per_fuel_liter: *weight_per_fuel_liter,
            benchmark_catch_value_per_fuel_liter: *catch_value_per_fuel_liter,
            benchmark_eeoi: *eeoi,
            benchmark_status: *status,
            benchmark_state_counter: *benchmark_state_counter,
        }
    }
}
