use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput,
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
        if trip.engines().is_empty() {
            return Ok(());
        }

        let track = adapter.trip_positions_with_manual(trip.trip_id).await?;

        let len = track.len();
        if len < 2 {
            return Ok(());
        }

        let mut estimated_fuel = 0.;
        let mut i = 0;

        while i < len {
            let Some((start_idx, start)) = track
                .iter()
                .enumerate()
                .skip(i)
                .find(|(_, v)| !v.covered_by_manual_fuel_entry)
            else {
                break;
            };
            let Some((end_idx, end)) = track
                .iter()
                .enumerate()
                .skip(start_idx + 1)
                .take_while(|(_, v)| !v.covered_by_manual_fuel_entry)
                .last()
            else {
                i = start_idx + 2;
                continue;
            };

            estimated_fuel +=
                end.cumulative_fuel_consumption_liter - start.cumulative_fuel_consumption_liter;
            i = end_idx + 1;
        }

        let overlapping_measurement_fuel = adapter
            .overlapping_measurment_fuel(trip.vessel_id, &trip.period)
            .await?;

        output.fuel_consumption_liter = Some(estimated_fuel + overlapping_measurement_fuel);

        Ok(())
    }
}
