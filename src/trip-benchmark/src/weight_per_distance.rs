use async_trait::async_trait;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
};

#[derive(Default)]
pub struct WeightPerDistance {}

#[async_trait]
impl TripBenchmark for WeightPerDistance {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::WeightPerDistance
    }

    async fn benchmark(
        &self,
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        let trips = adapter.trips_with_distance(vessel.fiskeridir.id).await?;

        let output = trips
            .into_iter()
            .map(|t| {
                let value = if t.distance == 0. {
                    0.
                } else {
                    t.total_weight / t.distance
                };

                TripBenchmarkOutput {
                    trip_id: t.id,
                    benchmark_id: TripBenchmarkId::WeightPerDistance,
                    value,
                    unrealistic: false,
                }
            })
            .collect();

        Ok(output)
    }
}
