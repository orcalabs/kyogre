use async_trait::async_trait;
use kyogre_core::{
    BenchmarkTrip, CoreResult, DateRange, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
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

        dbg!(&track.len());
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
                trip_cumulative_fuel_consumption: cumulative_fuel,
            },
        );

        let measurements = adapter
            .trip_fuel_measurements(trip.vessel_id, &trip.period)
            .await?;

        let mut measurement_diffs = vec![];
        let mut sum_diffs = 0.0;
        if let Some(start) = measurements.start_measurement_ts {
            measurement_diffs.push(DateRange::new(start, trip.period.start())?);
        }
        if let Some(end) = measurements.end_measurement_ts {
            measurement_diffs.push(DateRange::new(trip.period.end(), end)?);
        }

        dbg!(&measurement_diffs);

        for m in measurement_diffs {
            sum_diffs += estimate_fuel(
                &engines,
                trip.service_speed,
                trip.degree_of_electrification,
                adapter
                    .ais_vms_positions_with_haul(
                        trip.vessel_id,
                        trip.mmsi,
                        trip.call_sign.as_ref(),
                        &m,
                    )
                    .await?,
                &mut vec![],
                |_, _| {},
            );
        }

        dbg!(sum_diffs);

        adapter
            .update_trip_position_fuel_consumption(&fuel_updates)
            .await?;

        output.fuel_consumption = Some(
            dbg!(measurements.total_overlapping_fuel) + dbg!(estimated_fuel) - dbg!(sum_diffs),
        );

        Ok(())
    }
}
