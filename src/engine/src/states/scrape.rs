use crate::*;
use async_trait::async_trait;
use chrono::{Duration, NaiveTime, Utc};
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
            let limit = (Utc::now() - ais_area_window()).date_naive();

            if let Err(e) = shared_state
                .ais_pruner_inbound
                .prune_ais_vms_area(limit)
                .await
            {
                error!("failed to prune ais area: {e:?}");
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
                Schedule::Daily(NaiveTime::from_hms_opt(7, 0, 0).unwrap())
            }
            Environment::Local => Schedule::Periodic(Duration::hours(1)),
            Environment::Test => Schedule::Periodic(Duration::seconds(0)),
        }
    }
}
