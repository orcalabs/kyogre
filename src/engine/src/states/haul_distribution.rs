use crate::*;
use async_trait::async_trait;
use tracing::{event, Level};

pub struct HaulDistributionState;

#[async_trait]
impl machine::State for HaulDistributionState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        let database = shared_state.postgres_adapter();
        for b in &shared_state.haul_distributors {
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
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
