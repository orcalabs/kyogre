use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, DIESEL_LITER_CARBON_FACTOR, TripBenchmark, TripBenchmarkId,
    TripBenchmarkOutbound, TripBenchmarkOutput,
};

/// Computes the Carbon intensity for trips in the unit: `liter * DIESEL_LITER_CARBON_FACTOR / tonn`
#[derive(Default)]
pub struct CarbonIntensity;

#[async_trait]
impl TripBenchmark for CarbonIntensity {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::CarbonIntensity
    }

    async fn benchmark(
        &self,
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        output.carbon_intensity = carbon_intensity(
            output.fuel_consumption_liter,
            trip.distance,
            trip.total_catch_weight,
        );
        output.carbon_intensity_estimated_only = carbon_intensity(
            output.fuel_consumption_liter_estimated_only,
            trip.distance,
            trip.total_catch_weight,
        );

        Ok(())
    }
}

fn carbon_intensity(
    fuel_liter: Option<f64>,
    distance: Option<f64>,
    total_catch_weight: f64,
) -> Option<f64> {
    match (fuel_liter, distance) {
        (Some(fuel_liter), Some(distance))
            if fuel_liter > 0.0 && distance > 0.0 && total_catch_weight > 0.0 =>
        {
            Some((fuel_liter * DIESEL_LITER_CARBON_FACTOR) / (total_catch_weight / 1000.0))
        }
        _ => None,
    }
}
