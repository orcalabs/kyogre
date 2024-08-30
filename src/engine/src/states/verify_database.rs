use crate::SharedState;
use async_trait::async_trait;
use machine::Schedule;
use tracing::error;

pub struct VerifyDatabaseState;

#[async_trait]
impl machine::State for VerifyDatabaseState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        if let Err(e) = shared_state.verifier.verify_database().await {
            error!("verify database failed with error: {e:?}");
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
