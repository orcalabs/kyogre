use crate::{FuelEstimator, Result, Settings};
use chrono::{Duration, Utc};
use orca_core::Environment;
use postgres::PostgresAdapter;
use std::sync::Arc;

static RUN_INTERVAL: Duration = Duration::hours(5);

pub struct App {
    estimator: FuelEstimator,
    environment: Environment,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        if settings.environment == Environment::Local {
            postgres.do_migrations().await;
        }

        Self {
            estimator: FuelEstimator::new(settings.num_fuel_estimation_workers, Arc::new(postgres)),
            environment: settings.environment,
        }
    }

    pub async fn run(&self) -> Result<()> {
        match self.environment {
            Environment::Local
            | Environment::Production
            | Environment::Development
            | Environment::OnPremise => loop {
                if let Some(last_run) = self.estimator.last_run().await? {
                    let diff = Utc::now() - last_run;
                    if diff >= RUN_INTERVAL {
                        self.estimator.run().await?;
                    } else {
                        tokio::time::sleep(diff.to_std().unwrap()).await;
                        self.estimator.run().await?;
                    }
                } else {
                    self.estimator.run().await?;
                }
            },
            Environment::Test => self.estimator.run().await,
        }
    }
}
