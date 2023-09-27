use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use kyogre_core::*;
use chrono::{DateTime, Utc};

pub struct WeightPerHourInterval {
    pub from : DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub benchmark_id: VesselBenchmarkId,
}

#[async_trait]
impl VesselBenchmark for WeightPerHourInterval {
    fn benchmark_id(&self) -> VesselBenchmarkId {
        self.benchmark_id
    }
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<f64, BenchmarkError> {
        let trip_total = adapter
            .sum_trip_time_interval(vessel.fiskeridir.id, self.from, self.to)
            .await
            .change_context(BenchmarkError)?;
        let landing_total = adapter
            .sum_landing_weight_time_interval(vessel.fiskeridir.id, self.from, self.to)
            .await
            .change_context(BenchmarkError)?;

        Ok(match (trip_total, landing_total) {
            (Some(trip_total), Some(landing_total)) => {
                let hours = trip_total.num_seconds() as f64 / 3600.0;

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
