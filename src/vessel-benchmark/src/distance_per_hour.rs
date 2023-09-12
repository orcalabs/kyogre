use crate::{BenchmarkError, VesselBenchmark, VesselBenchmarkId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use kyogre_core::*;


#[derive(Default)]
pub struct DistancePerHour {}

#[async_trait]
impl VesselBenchmark for DistancePerHour {
    fn benchmark_id(&self) -> VesselBenchmarkId {
        VesselBenchmarkId::DistancePerHour
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
        let landing_total: Option<f64> = adapter
            .sum_landing_weight(vessel.fiskeridir.id)
            .await
            .change_context(BenchmarkError)?;

        Ok(match (trip_total) {
            (Some(trip_total)) => {
                trip_total.num_seconds() as f64 / 3600.0;
            }
            _ => 0.0,
        })
    

    }
}