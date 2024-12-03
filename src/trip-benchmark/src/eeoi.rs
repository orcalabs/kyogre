use async_trait::async_trait;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
    DIESEL_CARBON_FACTOR, METERS_TO_NAUTICAL_MILES,
};

/// Computes the EEOI for trips in the unit: `tonn / (tonn * nautical miles)`
#[derive(Default)]
pub struct Eeoi;

#[async_trait]
impl TripBenchmark for Eeoi {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::Eeoi
    }

    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        let trips = adapter
            .trips_without_eeoi_and_with_distance_and_fuel_consumption(vessel.fiskeridir.id)
            .await?;

        let output = trips
            .into_iter()
            .map(|t| {
                let eeoi = (t.fuel_consumption * DIESEL_CARBON_FACTOR)
                    / (t.total_weight * t.distance * METERS_TO_NAUTICAL_MILES)
                    / 1000.0;

                TripBenchmarkOutput {
                    trip_id: t.id,
                    benchmark_id: TripBenchmarkId::Eeoi,
                    value: eeoi,
                    unrealistic: false,
                }
            })
            .collect();

        Ok(output)
    }
}
