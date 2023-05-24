use crate::{BenchmarkError, VesselBenchmark, VesselBenchmarkId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use kyogre_core::*;

#[derive(Default)]
pub struct WeightPerHour {}

#[async_trait]
impl VesselBenchmark for WeightPerHour {
    fn benchmark_id(&self) -> VesselBenchmarkId {
        VesselBenchmarkId::WeightPerHour
    }
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<f64, BenchmarkError> {
        let trip_total = adapter
            .sum_trip_time(vessel.fiskeridir.id)
            .await
            .change_context(BenchmarkError)?;
        let landing_total = adapter
            .sum_landing_weight(vessel.fiskeridir.id)
            .await
            .change_context(BenchmarkError)?;

        Ok(match (trip_total, landing_total) {
            (Some(trip_total), Some(landing_total)) => {
                let hours = dbg!(trip_total.num_seconds()) as f64 / 3600.0;

                if hours == 0.0 || landing_total == 0.0 {
                    0.0
                } else {
                    landing_total / hours
                }
            }
            _ => 0.0,
        })
    }
}
