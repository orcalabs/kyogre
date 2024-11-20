use async_trait::async_trait;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
};

/// Computes the catch value (NOK) received per fuel (tonn) for a trip.
#[derive(Default)]
pub struct CatchValuePerFuel {}

#[async_trait]
impl TripBenchmark for CatchValuePerFuel {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::CatchValuePerFuel
    }

    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        let trips = adapter
            .trips_with_catch_value_and_fuel(vessel.fiskeridir.id)
            .await?;

        let output = trips
            .into_iter()
            .map(|t| {
                let value = if t.fuel_consumption == 0. {
                    0.
                } else {
                    t.total_catch_value / t.fuel_consumption
                };

                TripBenchmarkOutput {
                    trip_id: t.id,
                    benchmark_id: TripBenchmarkId::CatchValuePerFuel,
                    value,
                    unrealistic: false,
                }
            })
            .collect();

        Ok(output)
    }
}
