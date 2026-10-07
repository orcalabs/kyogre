use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput,
};

/// Computes the FUI for trips in the unit: `liter / tonn`
#[derive(Default)]
pub struct Fui;

#[async_trait]
impl TripBenchmark for Fui {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::Fui
    }

    async fn benchmark(
        &self,
        trip: &BenchmarkTrip,
        _adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        output.fui = fui(
            output.fuel_consumption_liter,
            trip.distance,
            trip.total_catch_weight,
        );
        output.fui_estimated_only = fui(
            output.fuel_consumption_liter_estimated_only,
            trip.distance,
            trip.total_catch_weight,
        );

        Ok(())
    }
}

fn fui(fuel_liter: Option<f64>, distance: Option<f64>, total_catch_weight: f64) -> Option<f64> {
    match (fuel_liter, distance) {
        (Some(fuel_liter), Some(distance))
            if fuel_liter > 0.0 && distance > 0.0 && total_catch_weight > 0.0 =>
        {
            Some(fuel_liter / (total_catch_weight / 1000.0))
        }
        _ => None,
    }
}
