use async_trait::async_trait;
use fuel_processor::estimate_fuel;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput, UpdateTripPositionFuel,
};

/// Computes fuel consumption for a trip in tonnes.
#[derive(Default)]
pub struct FuelConsumption;

#[async_trait]
impl TripBenchmark for FuelConsumption {
    fn benchmark_id(&self) -> TripBenchmarkId {
        TripBenchmarkId::FuelConsumption
    }

    async fn benchmark(
        &self,
        trip: &BenchmarkTrip,
        adapter: &dyn TripBenchmarkOutbound,
        output: &mut TripBenchmarkOutput,
    ) -> CoreResult<()> {
        let engines = trip.engines();
        if engines.is_empty() {
            return Ok(());
        }

        let track = adapter.track_of_trip_with_haul(trip.trip_id).await?;

        if track.len() < 2 {
            return Ok(());
        }

        let mut fuel_updates = Vec::with_capacity(track.len());

        let fuel_consumption_tonnes = estimate_fuel(
            &engines,
            trip.service_speed,
            trip.degree_of_electrification,
            track,
            &mut fuel_updates,
            |p, cumulative_fuel| UpdateTripPositionFuel {
                trip_id: trip.trip_id,
                timestamp: p.timestamp,
                position_type_id: p.position_type_id,
                trip_cumulative_fuel_consumption: cumulative_fuel,
            },
        );

        adapter
            .update_trip_position_fuel_consumption(&fuel_updates)
            .await?;

        output.fuel_consumption = Some(fuel_consumption_tonnes);

        Ok(())
    }
}
