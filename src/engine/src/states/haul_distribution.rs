use crate::*;
use tracing::{event, instrument, Level};

// Pending -> HaulDistribution
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, HaulDistribution> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, HaulDistribution> {
        val.inherit(HaulDistribution::default())
    }
}

// HaulDistribution -> Pending
impl<L, T> From<StepWrapper<L, T, HaulDistribution>> for StepWrapper<L, T, Pending> {
    fn from(val: StepWrapper<L, T, HaulDistribution>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

#[derive(Default)]
pub struct HaulDistribution;

impl<A, B> StepWrapper<A, SharedState<B>, HaulDistribution>
where
    B: Database,
{
    #[instrument(name = "haul_distribution_state", skip_all, fields(app.engine_state))]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record(
            "app.engine_state",
            EngineDiscriminants::HaulDistribution.as_ref(),
        );
        self.do_step().await;
        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }

    #[instrument(skip_all)]
    async fn do_step(&self) {
        let database = self.database();
        for b in self.haul_distributors() {
            if let Err(e) = b.distribute_hauls(database, database).await {
                event!(
                    Level::ERROR,
                    "failed to run haul distributor {}, err: {:?}",
                    b.haul_distributor_id(),
                    e
                );
            }
        }
    }
}