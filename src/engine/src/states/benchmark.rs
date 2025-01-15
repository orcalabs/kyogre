use crate::*;
use async_trait::async_trait;
use machine::Schedule;
use tracing::{error, instrument};

static BENCHMARK_COMMIT_SIZE: usize = 50;

pub struct BenchmarkState;

#[async_trait]
impl machine::State for BenchmarkState {
    type SharedState = SharedState;

    #[instrument(skip_all)]
    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        if let Err(e) = state_impl(&shared_state).await {
            error!("failed to run benchmark state: {e:?}");
        }
        shared_state
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

#[instrument(skip_all)]
async fn state_impl(shared_state: &SharedState) -> Result<(), Error> {
    let trips = shared_state.benchmark_outbound.trips_to_benchmark().await?;

    let mut outputs = Vec::with_capacity(trips.len());

    for (i, t) in trips.into_iter().enumerate() {
        match benchmark_trip(shared_state, &t).await {
            Ok(o) => {
                outputs.push(o);
            }
            Err(e) => {
                error!(
                    "failed to run benchmarks for trip '{}', err: {e:?}",
                    t.trip_id
                );
            }
        }

        if i % BENCHMARK_COMMIT_SIZE == 0 && i > 0 {
            shared_state.benchmark_inbound.add_output(&outputs).await?;
            outputs.clear();
        }
    }

    if !outputs.is_empty() {
        shared_state.benchmark_inbound.add_output(&outputs).await?;
    }

    Ok(())
}

#[instrument(skip_all)]
async fn benchmark_trip(
    shared_state: &SharedState,
    trip: &BenchmarkTrip,
) -> Result<TripBenchmarkOutput, Error> {
    let mut output = TripBenchmarkOutput {
        trip_id: trip.trip_id,
        weight_per_hour: None,
        weight_per_distance: None,
        fuel_consumption: None,
        weight_per_fuel: None,
        catch_value_per_fuel: None,
        eeoi: None,
        status: ProcessingStatus::Successful,
    };

    for b in &shared_state.benchmarks {
        b.benchmark(trip, shared_state.benchmark_outbound.as_ref(), &mut output)
            .await?;
    }

    Ok(output)
}
