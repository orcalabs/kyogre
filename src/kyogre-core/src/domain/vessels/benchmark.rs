use std::collections::HashMap;

use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::VesselBenchmarkId;

// trait BenchmarkPort: VesselBenchmarkOutbound + VesselBenchmarkInbound {}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VesselBenchmarks {
    /// Time unit is in minutes
    pub fishing_time: Option<Benchmark>,
    /// Distance unit is in meters
    pub fishing_distance: Option<Benchmark>,
    /// Time unit is in minutes
    pub trip_time: Option<Benchmark>,
    pub landings: Option<Benchmark>,
    pub ers_dca: Option<Benchmark>,
    pub cumulative_landings: Vec<CumulativeLandings>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Benchmark {
    pub average: f64,
    pub average_followers: f64,
    pub recent_trips: Vec<BenchmarkEntry>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkEntry {
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub trip_start: DateTime<Utc>,
    pub value: f64,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CumulativeLandings {
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub month: chrono::Month,
    pub species_fiskeridir_id: u32,
    pub weight: f64,
    pub cumulative_weight: f64,
}

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
                    error!("failed to run benchmark {id}, err: {e:?}");
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

impl PartialEq<(&TripDetailed, f64)> for &BenchmarkEntry {
    fn eq(&self, other: &(&TripDetailed, f64)) -> bool {
        self.value as i64 == other.1 as i64
            && self.fiskeridir_vessel_id == other.0.fiskeridir_vessel_id
            && self.trip_start == other.0.period.start()
    }
}

impl PartialEq<&BenchmarkEntry> for (&TripDetailed, f64) {
    fn eq(&self, other: &&BenchmarkEntry) -> bool {
        other.eq(self)
    }
}
