use crate::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;
use strum::Display;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Deserialize_repr, Display)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub enum TripBenchmarkId {
    WeightPerHour = 1,
    Sustainability = 2,
    WeightPerDistance = 3,
    FuelConsumption = 4,
    WeightPerFuel = 5,
    CatchValuePerFuel = 6,
    Eeoi = 7,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct AverageTripBenchmarks {
    pub weight_per_hour: Option<f64>,
    pub weight_per_distance: Option<f64>,
    #[serde(rename = "weightPerFuel")]
    pub weight_per_fuel_liter: Option<f64>,
    #[serde(rename = "catchValuePerFuel")]
    pub catch_value_per_fuel_liter: Option<f64>,
    #[serde(rename = "fuelConsumption")]
    pub fuel_consumption_liter: Option<f64>,
    // TODO
    // pub sustainability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripWithBenchmark {
    pub id: TripId,
    pub period: DateRange,
    pub period_precision: Option<DateRange>,
    pub weight_per_hour: Option<f64>,
    pub weight_per_distance: Option<f64>,
    pub weight_per_fuel_liter: Option<f64>,
    pub catch_value_per_fuel_liter: Option<f64>,
    pub fuel_consumption_liter: Option<f64>,
    pub eeoi: Option<f64>,
    // TODO
    // pub sustainability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateTripPositionFuel {
    pub trip_id: TripId,
    pub timestamp: DateTime<Utc>,
    pub position_type_id: PositionType,
    pub trip_cumulative_fuel_consumption_liter: f64,
}

#[derive(Debug, Clone)]
pub struct TripBenchmarkOutput {
    pub trip_id: TripId,
    pub weight_per_hour: Option<f64>,
    pub weight_per_distance: Option<f64>,
    pub fuel_consumption_liter: Option<f64>,
    pub weight_per_fuel_liter: Option<f64>,
    pub catch_value_per_fuel_liter: Option<f64>,
    pub eeoi: Option<f64>,
    pub status: ProcessingStatus,
}

#[async_trait]
pub trait TripBenchmark: Send + Sync {
    fn benchmark_id(&self) -> TripBenchmarkId;
    async fn benchmark(
        &self,
        trip: &BenchmarkTrip,
        adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()>;
}

impl From<TripBenchmarkId> for i32 {
    fn from(value: TripBenchmarkId) -> Self {
        value as i32
    }
}
