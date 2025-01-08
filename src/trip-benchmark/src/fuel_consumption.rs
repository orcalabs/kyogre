use async_trait::async_trait;
use fuel_processor::estimate_fuel;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput,
    UpdateTripPositionFuel, Vessel,
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
        vessel: &Vessel,
        adapter: &dyn TripBenchmarkOutbound,
    ) -> CoreResult<Vec<TripBenchmarkOutput>> {
        let Some(sfc) = vessel.sfc() else {
            return Ok(vec![]);
        };
        let Some(engine_power_kw) = vessel.engine_power_kw() else {
            return Ok(vec![]);
        };

        let trips = adapter
            .trips_without_fuel_consumption(vessel.fiskeridir.id)
            .await?;

        let mut output = Vec::with_capacity(trips.len());
        let mut fuel_updates = Vec::with_capacity(trips.len());

        for id in trips {
            let track = adapter.track_of_trip_with_haul(id).await?;

            if track.len() < 2 {
                continue;
            }

            let fuel_consumption_tonnes = estimate_fuel(
                sfc,
                engine_power_kw,
                track,
                &mut fuel_updates,
                |p, cumulative_fuel| UpdateTripPositionFuel {
                    trip_id: id,
                    timestamp: p.timestamp,
                    position_type_id: p.position_type_id,
                    trip_cumulative_fuel_consumption: cumulative_fuel,
                },
            );

            adapter
                .update_trip_position_fuel_consumption(&fuel_updates)
                .await?;
            fuel_updates.clear();

            output.push(TripBenchmarkOutput {
                trip_id: id,
                benchmark_id: TripBenchmarkId::FuelConsumption,
                value: fuel_consumption_tonnes,
                unrealistic: false,
            });
        }

        Ok(output)
    }
}
