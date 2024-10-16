use async_trait::async_trait;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
};

/// Computes the weight (kg) caught per fuel (tonn) for a trip.
#[derive(Default)]
pub struct WeightPerFuel {}

#[async_trait]
impl TripBenchmark for WeightPerFuel {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::WeightPerFuel
    }

    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        let trips = adapter
            .trips_with_weight_and_fuel(vessel.fiskeridir.id)
            .await?;

        let output = trips
            .into_iter()
            .map(|t| {
                let value = if t.fuel_consumption == 0. {
                    0.
                } else {
                    t.total_weight / t.fuel_consumption
                };

                TripBenchmarkOutput {
                    trip_id: t.id,
                    benchmark_id: TripBenchmarkId::WeightPerFuel,
                    value,
                    unrealistic: false,
                }
            })
            .collect();

        Ok(output)
    }
}
