use crate::*;
use async_trait::async_trait;
use machine::Schedule;
use tracing::error;

pub struct BenchmarkState;

#[async_trait]
impl machine::State for BenchmarkState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        for b in &shared_state.benchmarks {
            if let Err(e) = b
                .produce_and_store_benchmarks(
                    shared_state.benchmark_inbound.as_ref(),
                    shared_state.benchmark_outbound.as_ref(),
                )
                .await
            {
                error!("failed to run benchmark {}, err: {e:?}", b.benchmark_id());
            }
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
