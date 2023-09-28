use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::*;

pub struct WeightPerDistanceInterval {
    pub from : DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub benchmark_id: VesselBenchmarkId,
}

#[async_trait]
impl VesselBenchmark for WeightPerDistanceInterval {
    fn benchmark_id(&self) -> VesselBenchmarkId {
        self.benchmark_id
    }
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<f64, BenchmarkError> {
        let trip_total = adapter
            .sum_trip_distance_time_interval(vessel.fiskeridir.id, self.from, self.to)
            .await
            .change_context(BenchmarkError)?;
        let landing_total = adapter
            .sum_landing_weight_time_interval(vessel.fiskeridir.id, self.from, self.to)
            .await
            .change_context(BenchmarkError)?;

        Ok(match (trip_total, landing_total) {
            (Some(trip_total), Some(landing_total)) => {
                if trip_total == 0.0 || landing_total == 0.0 {
                    0.0
                } else {
                    landing_total / trip_total
                }
            }
            _ => 0.0,
        })
    }
}
