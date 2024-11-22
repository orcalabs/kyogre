use kyogre_core::{TripBenchmarkId, TripBenchmarkStatus, TripId};
use unnest_insert::UnnestInsert;

use crate::queries::{type_to_i32, type_to_i64};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "trip_benchmark_outputs",
    conflict = "trip_id,trip_benchmark_id",
    update_all
)]
pub struct TripBenchmarkOutput {
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "type_to_i64")]
    pub trip_id: TripId,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub trip_benchmark_id: TripBenchmarkId,
    pub output: f64,
    pub unrealistic: bool,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub status: TripBenchmarkStatus,
}

impl From<&kyogre_core::TripBenchmarkOutput> for TripBenchmarkOutput {
    fn from(v: &kyogre_core::TripBenchmarkOutput) -> Self {
        let kyogre_core::TripBenchmarkOutput {
            trip_id,
            benchmark_id,
            value,
            unrealistic,
        } = v;

        Self {
            trip_id: *trip_id,
            trip_benchmark_id: *benchmark_id,
            output: *value,
            unrealistic: *unrealistic,
            status: TripBenchmarkStatus::MustRefresh,
        }
    }
}
