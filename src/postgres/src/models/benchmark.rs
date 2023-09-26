use bigdecimal::BigDecimal;
use error_stack::Report;
use kyogre_core::{VesselBenchmarkId, FiskeridirVesselId};
use num_traits::ToPrimitive;
use serde::Deserialize;

use crate::error::PostgresError;

#[derive(Debug, Deserialize)]
pub struct Benchmarks {
    pub vessel_id : i64,
    pub benchmark_id: i64,
    pub output : BigDecimal,
}

impl TryFrom<Benchmarks> for kyogre_core::Benchmark{
    type Error = Report<PostgresError>;

    fn try_from(value: Benchmarks) -> Result<Self, Self::Error> {
        Ok(Self { 
            vessel_id: FiskeridirVesselId(value.vessel_id), 
            benchmark_id: match value.benchmark_id {
                1 => VesselBenchmarkId::WeightPerHour,
                2 => VesselBenchmarkId::WeightPerDistance,
                _ => {
                    return Err(PostgresError::DataConversion.into());
                }
            }, 
            output: value.output.to_f64(), 
        })
    }
}