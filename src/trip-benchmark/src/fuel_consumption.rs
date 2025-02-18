use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput, UpdateTripPositionFuel,
};
use processors::estimate_fuel;

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

        let track = adapter
            .ais_vms_positions_with_haul_and_manual(
                trip.vessel_id,
                trip.mmsi,
                trip.call_sign.as_ref(),
                &trip.period,
                trip.trip_id,
            )
            .await?;

        if track.len() < 2 {
            return Ok(());
        }

        let mut fuel_updates = Vec::with_capacity(track.len());

        let estimated_fuel = estimate_fuel(
            &engines,
            trip.service_speed,
            trip.degree_of_electrification,
            track,
            &mut fuel_updates,
            |p, cumulative_fuel| UpdateTripPositionFuel {
                trip_id: trip.trip_id,
                timestamp: p.timestamp,
                position_type_id: p.position_type_id,
                trip_cumulative_fuel_consumption_liter: cumulative_fuel,
            },
        );

        let overlapping_measurement_fuel = adapter
            .overlapping_measurment_fuel(trip.vessel_id, &trip.period)
            .await?;

        adapter
            .update_trip_position_fuel_consumption(&fuel_updates)
            .await?;

        output.fuel_consumption_liter =
            Some(estimated_fuel.fuel_liter + overlapping_measurement_fuel);

        Ok(())
    }
}
