use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput,
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
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        output.weight_per_fuel = match output.fuel_consumption {
            Some(fuel) if fuel > 0.0 => Some(trip.total_catch_weight / fuel),
            _ => None,
        };

        Ok(())
    }
}
