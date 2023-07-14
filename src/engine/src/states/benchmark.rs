use crate::*;
use tracing::{event, instrument, Level};

// Pending -> Benchmark
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, Benchmark> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, Benchmark> {
        val.inherit(Benchmark)
    }
}

// Benchmark -> HaulDistribution
impl<L, T> From<StepWrapper<L, T, Benchmark>> for StepWrapper<L, T, HaulDistribution> {
    fn from(val: StepWrapper<L, T, Benchmark>) -> StepWrapper<L, T, HaulDistribution> {
        val.inherit(HaulDistribution)
    }
}

#[derive(Default)]
pub struct Benchmark;

impl<A, B> StepWrapper<A, SharedState<B>, Benchmark>
where
    B: Database,
{
    #[instrument(name = "benchmark_state", skip_all, fields(app.engine_state))]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current()
            .record("app.engine_state", EngineDiscriminants::Benchmark.as_ref());
        self.do_step().await;
        Engine::HaulDistribution(StepWrapper::<A, SharedState<B>, HaulDistribution>::from(
            self,
        ))
    }

    #[instrument(skip_all)]
    async fn do_step(&self) {
        let database = self.database();
        for b in self.vessel_benchmarks() {
            if let Err(e) = b.produce_and_store_benchmarks(database, database).await {
                event!(
                    Level::ERROR,
                    "failed to run benchmark {}, err: {:?}",
                    b.benchmark_id(),
                    e
                );
            }
        }
    }
}
