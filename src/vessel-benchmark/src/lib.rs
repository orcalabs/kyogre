use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::{Context, Result, ResultExt};
use kyogre_core::*;
use tracing::{event, Level};

mod weight_per_hour;

mod weight_per_date;
pub use weight_per_hour::*;
pub use weight_per_date::*;

trait BenchmarkPort: VesselBenchmarkOutbound + VesselBenchmarkInbound {}

#[async_trait]
pub trait VesselBenchmark: Send + Sync {
    fn benchmark_id(&self) -> VesselBenchmarkId;
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<f64, BenchmarkError>;
    async fn produce_and_store_benchmarks(
        &self,
        input_adapter: &dyn VesselBenchmarkInbound,
        output_adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<(), BenchmarkError> {
        let id = self.benchmark_id();
        let vessels = output_adapter
            .vessels()
            .await
            .change_context(BenchmarkError)?
            .into_iter()
            .map(|v| (v.fiskeridir.id, v))
            .collect::<HashMap<FiskeridirVesselId, Vessel>>();

        let mut outputs = Vec::with_capacity(vessels.len());
        for v in vessels.into_values() {
            match self.benchmark(&v, output_adapter).await {
                Ok(value) => {
                    outputs.push(VesselBenchmarkOutput {
                        benchmark_id: id,
                        vessel_id: v.fiskeridir.id,
                        value,
                    });
                }
                Err(e) => {
                    event!(Level::ERROR, "failed to run benchmark {}, err: {:?}", id, e);
                }
            }
        }

        input_adapter
            .add_output(outputs)
            .await
            .change_context(BenchmarkError)?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct BenchmarkError;

impl Context for BenchmarkError {}

impl std::fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occured while running a benchmark")
    }
}
