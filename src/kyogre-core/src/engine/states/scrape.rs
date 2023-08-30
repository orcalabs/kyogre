use crate::*;
use async_trait::async_trait;
use chrono::{Duration, NaiveTime};
use machine::Schedule;
use orca_core::Environment;
use tracing::{event, Level};

pub struct ScrapeState;

#[async_trait]
impl machine::State for ScrapeState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        if let Some(scraper) = &shared_state.scraper {
            scraper.run().await;
            if let Err(e) = shared_state.matrix_cache.increment().await {
                event!(
                    Level::ERROR,
                    "failed to increment cache data version: {:?}",
                    e
                );
            }
        }
    }
    fn schedule(&self) -> Schedule {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .expect("failed to parse APP_ENVIRONMENT");

        match environment {
            Environment::Production
            | Environment::Staging
            | Environment::Development
            | Environment::Test => Schedule::Daily(NaiveTime::from_hms_opt(7, 0, 0).unwrap()),
            Environment::Local => Schedule::Periodic(Duration::hours(1)),
        }
    }
}
