use chrono::{DateTime, Utc};
use kyogre_core::{DateRange, TripBenchmarkId, TripId};
use sqlx::postgres::types::PgRange;
use unnest_insert::UnnestInsert;

use crate::{
    error::Error,
    queries::{type_to_i32, type_to_i64},
};

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripWithBenchmark {
    pub trip_id: TripId,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub weight_per_hour: f64,
    // TODO
    // pub sustainability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TripWithTotalLivingWeight {
    pub trip_id: TripId,
    pub period: PgRange<DateTime<Utc>>,
    pub period_precision: Option<PgRange<DateTime<Utc>>>,
    pub total_living_weight: f64,
}

impl From<kyogre_core::TripBenchmarkOutput> for TripBenchmarkOutput {
    fn from(v: kyogre_core::TripBenchmarkOutput) -> Self {
        Self {
            trip_id: v.trip_id,
            trip_benchmark_id: v.benchmark_id,
            output: v.value,
            unrealistic: v.unrealistic,
        }
    }
}

impl TryFrom<TripWithTotalLivingWeight> for kyogre_core::TripWithTotalLivingWeight {
    type Error = Error;

    fn try_from(v: TripWithTotalLivingWeight) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.trip_id,
            period: DateRange::try_from(v.period)?,
            period_precision: v.period_precision.map(TryFrom::try_from).transpose()?,
            total_living_weight: v.total_living_weight,
        })
    }
}

impl TryFrom<TripWithBenchmark> for kyogre_core::TripWithBenchmark {
    type Error = Error;

    fn try_from(v: TripWithBenchmark) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.trip_id,
            period: DateRange::try_from(v.period)?,
            period_precision: v.period_precision.map(TryFrom::try_from).transpose()?,
            weight_per_hour: v.weight_per_hour,
        })
    }
}
