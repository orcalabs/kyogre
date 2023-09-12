use crate::SharedState;
use async_trait::async_trait;
use machine::Schedule;
use tracing::{event, Level};

pub struct UpdateDatabaseViewsState;

#[async_trait]
impl machine::State for UpdateDatabaseViewsState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        if let Err(e) = shared_state.refresher.refresh().await {
            event!(Level::ERROR, "failed to update database views {:?}", e);
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
