use async_trait::async_trait;
use chrono::{DateTime, Utc};
use kyogre_core::{
    CoreResult, Mean, PositionType, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound,
    TripBenchmarkOutput, UpdateTripPositionFuel, Vessel,
};

const HP_TO_KW: f64 = 0.745699872;

/// Computes fuel consumption for a trip in tonnes.
#[derive(Default)]
pub struct FuelConsumption {}

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
        // Specific Fuel Consumption
        // Source: https://wwwcdn.imo.org/localresources/en/OurWork/Environment/Documents/Fourth%20IMO%20GHG%20Study%202020%20-%20Full%20report%20and%20annexes.pdf
        //         Annex B.2, Table 4
        let sfc: f64 = match vessel.fiskeridir.engine_building_year {
            Some(v) => match v {
                ..1984 => [205., 190., 215., 200., 225., 210.],
                1984..2001 => [185., 175., 195., 185., 205., 190.],
                2001.. => [175., 165., 185., 175., 195., 185.],
            }
            .into_iter()
            .mean()
            .unwrap(),
            None => return Ok(vec![]),
        };
        let engine_power_kw = match vessel.fiskeridir.engine_power {
            Some(v) => v as f64 * HP_TO_KW,
            None => return Ok(vec![]),
        };

        let trips = adapter
            .trips_without_fuel_consumption(vessel.fiskeridir.id)
            .await?;

        let mut output = Vec::with_capacity(trips.len());
        let mut fuel_updates = Vec::with_capacity(trips.len());

        struct Item {
            speed: Option<f64>,
            timestamp: DateTime<Utc>,
            position_type_id: PositionType,
        }
        struct State {
            kwh: f64,
            prev: Item,
        }

        for id in trips {
            let track = adapter.track_of_trip(id).await?;

            if track.len() < 2 {
                continue;
            }

            let mut iter = track.into_iter().map(|v| Item {
                speed: v.speed,
                timestamp: v.timestamp,
                position_type_id: v.position_type,
            });

            let state = State {
                kwh: 0.,
                // `unwrap` is safe due to `len() < 2` check above
                prev: iter.next().unwrap(),
            };

            let result = iter.fold(state, |mut state, v| {
                let speed = match (state.prev.speed, v.speed) {
                    (Some(a), Some(b)) => (a + b) / 2.,
                    (Some(a), None) => a,
                    (None, Some(b)) => b,
                    (None, None) => return state,
                };

                // TODO: Currently using surrogate value from:
                // https://www.epa.gov/system/files/documents/2023-01/2020NEI_C1C2_Documentation.pdf
                // Table 3. C1C2 Propulsive Power and Load Factor Surrogates
                let speed_service = 12.;

                let load_factor = ((speed / speed_service).powf(3.) * 0.85).clamp(0., 0.98);

                state.kwh += load_factor
                    * engine_power_kw
                    * (v.timestamp - state.prev.timestamp).num_milliseconds() as f64
                    / 3_600_000.;

                fuel_updates.push(UpdateTripPositionFuel {
                    trip_id: id,
                    timestamp: v.timestamp,
                    position_type_id: v.position_type_id,
                    trip_cumulative_fuel_consumption: sfc * state.kwh / 1_000_000.,
                });

                state.prev = v;
                state
            });

            let fuel_consumption_tonnes = sfc * result.kwh / 1_000_000.;

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
