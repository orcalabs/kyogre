use std::collections::HashMap;

use async_trait::async_trait;
use serde_repr::Deserialize_repr;
use strum::Display;
use tracing::error;

use crate::*;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize_repr, Display)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum TripBenchmarkId {
    WeightPerHour = 1,
    Sustainability = 2,
    WeightPerDistance = 3,
    FuelConsumption = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripWithBenchmark {
    pub id: TripId,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub weight_per_hour: Option<f64>,
    pub weight_per_distance: Option<f64>,
    pub fuel_consumption: Option<f64>,
    // TODO
    // pub sustainability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripBenchmarkOutput {
    pub trip_id: TripId,
    pub benchmark_id: TripBenchmarkId,
    pub value: f64,
    pub unrealistic: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripWithTotalWeight {
    pub id: TripId,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub total_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripWithDistance {
    pub id: TripId,
    pub distance: f64,
    pub total_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripSustainabilityMetric {
    pub id: TripId,
    pub weight_per_hour: f64,
    pub weight_per_distance: f64,
}

#[async_trait]
pub trait TripBenchmark: Send + Sync {
    fn benchmark_id(&self) -> TripBenchmarkId;
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>>;

    async fn produce_and_store_benchmarks(
        &self,
        input_adapter: &dyn TripBenchmarkInbound,
        output_adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<()> {
        let id = self.benchmark_id();

        let vessels = output_adapter
            .vessels()
            .await?
            .into_iter()
            .map(|v| (v.fiskeridir.id, v))
            .collect::<HashMap<FiskeridirVesselId, Vessel>>();

        for v in vessels.into_values() {
            match self.benchmark(&v, output_adapter).await {
                Ok(outputs) => {
                    if let Err(e) = input_adapter.add_output(outputs).await {
                        error!("failed to persist benchmark outputs for {id}, err: {e:?}");
                    }
                }
                Err(e) => {
                    error!("failed to run benchmark {id}, err: {e:?}");
                }
            }
        }

        Ok(())
    }
}

impl From<TripBenchmarkId> for i32 {
    fn from(value: TripBenchmarkId) -> Self {
        value as i32
    }
}
