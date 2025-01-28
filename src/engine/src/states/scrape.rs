use crate::*;
use async_trait::async_trait;
use chrono::{Duration, NaiveTime};
use machine::Schedule;
use orca_core::Environment;
use tracing::error;

pub struct ScrapeState;

#[async_trait]
impl machine::State for ScrapeState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: Self::SharedState) -> Self::SharedState {
        if let Some(scraper) = &shared_state.scraper {
            scraper.run().await;
            if let Err(e) = shared_state.matrix_cache.increment().await {
                error!("failed to increment cache data version: {e:?}");
            }
        }

        shared_state
    }
    fn schedule(&self) -> Schedule {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap_or("test".into())
            .try_into()
            .unwrap();

        match environment {
            Environment::Production | Environment::OnPremise | Environment::Development => {
                Schedule::Daily(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            }
            Environment::Local => Schedule::Periodic(Duration::hours(1)),
            Environment::Test => Schedule::Periodic(Duration::seconds(0)),
        }
    }
}
