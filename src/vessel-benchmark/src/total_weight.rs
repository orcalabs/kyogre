use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::*;

pub struct TotalWeightInterval {
    pub from : DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub benchmark_id: VesselBenchmarkId,
}

#[async_trait]
impl VesselBenchmark for TotalWeightInterval {
    fn benchmark_id(&self) -> VesselBenchmarkId {
        self.benchmark_id
    }
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<f64, BenchmarkError> {
        let landing_total = adapter
            .sum_landing_weight_time_interval(vessel.fiskeridir.id, self.from, self.to)
            .await
            .change_context(BenchmarkError)?;

        Ok(match landing_total {
            Some(landing_total) => {
                landing_total
            }
            _ => 0.0,
        })
    }
}
