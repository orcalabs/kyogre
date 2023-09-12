use crate::{BenchmarkError, VesselBenchmark, VesselBenchmarkId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use kyogre_core::*;
use chrono::{DateTime, Duration, Utc};

#[derive(Default)]
pub struct WeightPerDate {}

#[async_trait]
impl VesselBenchmark for WeightPerDate {
    fn benchmark_id(&self) -> VesselBenchmarkId {
        VesselBenchmarkId::WeightPerDate
    }
    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn VesselBenchmarkOutbound,
    ) -> Result<f64, BenchmarkError> {
        // find todays date
        let days_since :i64 =  1;
        let today : DateTime<Utc> = Utc::now() - Duration::days(days_since as i64);


        let landing_total = adapter
            .sum_landing_weight_on_date(vessel.fiskeridir.id,today)
            .await
            .change_context(BenchmarkError)?;

        Ok(match landing_total {
            Some(landing_total) => {

                let hours =  Duration::days(days_since).num_seconds() as f64 / 3600.0;

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

