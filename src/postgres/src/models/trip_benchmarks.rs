use crate::queries::{type_to_i32, type_to_i64};
use kyogre_core::{ProcessingStatus, TripId};
use unnest_insert::UnnestUpdate;

#[derive(Debug, Clone, UnnestUpdate)]
#[unnest_update(table_name = "trips_detailed")]
pub struct TripBenchmarkOutput {
    #[unnest_update(id, sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub trip_id: TripId,
    pub benchmark_weight_per_hour: Option<f64>,
    pub benchmark_weight_per_distance: Option<f64>,
    pub benchmark_fuel_consumption: Option<f64>,
    pub benchmark_weight_per_fuel: Option<f64>,
    pub benchmark_catch_value_per_fuel: Option<f64>,
    pub benchmark_eeoi: Option<f64>,
    #[unnest_update(sql_type = "INT", type_conversion = "type_to_i32")]
    pub benchmark_status: ProcessingStatus,
}

impl From<&kyogre_core::TripBenchmarkOutput> for TripBenchmarkOutput {
    fn from(v: &kyogre_core::TripBenchmarkOutput) -> Self {
        let kyogre_core::TripBenchmarkOutput {
            trip_id,
            weight_per_hour: benchmark_weight_per_hour,
            weight_per_distance: benchmark_weight_per_distance,
            fuel_consumption: benchmark_fuel_consumption,
            weight_per_fuel: benchmark_weight_per_fuel,
            catch_value_per_fuel: benchmark_catch_value_per_fuel,
            eeoi: benchmark_eeoi,
            status: benchmark_status,
        } = v;

        Self {
            trip_id: *trip_id,
            benchmark_weight_per_hour: *benchmark_weight_per_hour,
            benchmark_weight_per_distance: *benchmark_weight_per_distance,
            benchmark_fuel_consumption: *benchmark_fuel_consumption,
            benchmark_weight_per_fuel: *benchmark_weight_per_fuel,
            benchmark_catch_value_per_fuel: *benchmark_catch_value_per_fuel,
            benchmark_eeoi: *benchmark_eeoi,
            benchmark_status: *benchmark_status,
        }
    }
}
