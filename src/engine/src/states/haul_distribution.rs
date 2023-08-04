use crate::*;
use orca_statemachine::Pending;
use tracing::{event, instrument, Level};

// Pending -> HaulDistribution
impl<L: TransitionLog, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, HaulDistribution> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, HaulDistribution> {
        val.inherit(HaulDistribution)
    }
}

// HaulDistribution -> TripDistance
impl<L: TransitionLog, T> From<StepWrapper<L, T, HaulDistribution>>
    for StepWrapper<L, T, TripDistance>
{
    fn from(val: StepWrapper<L, T, HaulDistribution>) -> StepWrapper<L, T, TripDistance> {
        val.inherit(TripDistance)
    }
}

#[derive(Default)]
pub struct HaulDistribution;

impl<A: TransitionLog, B: Database> StepWrapper<A, SharedState<B>, HaulDistribution> {
    #[instrument(name = "haul_distribution_state", skip_all, fields(app.engine_state))]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record(
            "app.engine_state",
            EngineDiscriminants::HaulDistribution.as_ref(),
        );
        self.do_step().await;
        Engine::TripDistance(StepWrapper::<A, SharedState<B>, TripDistance>::from(self))
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
