use crate::*;
use async_trait::async_trait;
use chrono::NaiveTime;
use tracing::{event, Level};

pub struct ScrapeState;

#[async_trait]
impl machine::State for ScrapeState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        shared_state.scraper.run().await;
        if let Err(e) = shared_state.postgres_adapter().increment().await {
            event!(
                Level::ERROR,
                "failed to increment cache data version: {:?}",
                e
            );
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Daily(NaiveTime::from_hms_opt(7, 0, 0).unwrap())
    }
}
