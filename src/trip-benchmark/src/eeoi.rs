use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput, DIESEL_CARBON_FACTOR, METERS_TO_NAUTICAL_MILES,
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
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        output.eeoi = match (output.fuel_consumption, trip.distance) {
            (Some(fuel), Some(distance))
                if fuel > 0.0 && distance > 0.0 && trip.total_catch_weight > 0.0 =>
            {
                Some(
                    (fuel * DIESEL_CARBON_FACTOR)
                        / (trip.total_catch_weight * distance * METERS_TO_NAUTICAL_MILES)
                        / 1000.0,
                )
            }
            _ => None,
        };

        Ok(())
    }
}
