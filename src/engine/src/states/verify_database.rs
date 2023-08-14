use crate::SharedState;
use async_trait::async_trait;
use kyogre_core::VerificationOutbound;
use machine::Schedule;
use tracing::{event, Level};

pub struct VerifyDatabaseState;

#[async_trait]
impl machine::State for VerifyDatabaseState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        if let Err(e) = shared_state.database.verify_database().await {
            event!(Level::ERROR, "verify database failed with error: {:?}", e);
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
