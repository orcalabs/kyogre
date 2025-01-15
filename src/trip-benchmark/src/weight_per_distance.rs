use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput,
};

/// Computes the weight (kg) caught per distance (meter) for a trip.
#[derive(Default)]
pub struct WeightPerDistance {}

#[async_trait]
impl TripBenchmark for WeightPerDistance {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::WeightPerDistance
    }

    async fn benchmark(
        &self,
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        output.weight_per_distance = match trip.distance {
            Some(distance) if distance > 0.0 => Some(trip.total_catch_weight / distance),
            _ => None,
        };

        Ok(())
    }
}
