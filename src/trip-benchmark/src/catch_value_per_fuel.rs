use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput,
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
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        output.catch_value_per_fuel = match output.fuel_consumption {
            Some(fuel) if fuel > 0.0 => Some(trip.total_catch_value / fuel),
            _ => None,
        };

        Ok(())
    }
}
