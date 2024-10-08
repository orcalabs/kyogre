use async_trait::async_trait;
use geoutils::Location;
use kyogre_core::{
    CoreResult, TripBenchmark, TripBenchmarkId, TripBenchmarkOutbound, TripBenchmarkOutput, Vessel,
};
use tracing::error;

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
        let engine_power = match vessel.fiskeridir.engine_power {
            Some(v) => v as f64,
            None => return Ok(vec![]),
        };

        let trips = adapter
            .trips_without_fuel_consumption(vessel.fiskeridir.id)
            .await?;

        let mut output = Vec::with_capacity(trips.len());

        struct Item {
            loc: Location,
            speed: Option<f64>,
        }
        struct State {
            acc: f64,
            prev: Item,
        }

        for id in trips {
            let track = adapter.track_of_trip(id).await?;

            if track.len() < 2 {
                continue;
            }

            let mut iter = track.into_iter().map(|v| Item {
                loc: Location::new_const(v.latitude, v.longitude),
                speed: v.speed,
            });

            let state = State {
                acc: 0.,
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

                match state.prev.loc.distance_to(&v.loc) {
                    Ok(d) => {
                        // TODO: What to do when speed is zero since ship might still be running the engine?
                        state.acc += d.meters() * speed * engine_power;
                        state.prev = v;
                    }
                    Err(e) => {
                        error!(
                            "failed to compute distance from {:?} to {:?}, trip: {id}, err: {e:?}",
                            state.prev.loc, v.loc,
                        );
                    }
                }
                state
            });

            output.push(TripBenchmarkOutput {
                trip_id: id,
                benchmark_id: TripBenchmarkId::FuelConsumption,
                value: result.acc,
                unrealistic: false, // TODO
            });
        }

        Ok(output)
    }
}
