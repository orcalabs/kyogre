use crate::{FuelEstimator, LiveFuel, Result, Settings};
use orca_core::Environment;
use postgres::PostgresAdapter;
use std::sync::Arc;
use tokio::task::JoinSet;

pub struct App {
    estimator: FuelEstimator,
    live_fuel: LiveFuel,
    environment: Environment,
}

impl App {
    pub async fn build(settings: &Settings) -> Self {
        let postgres = Arc::new(PostgresAdapter::new(&settings.postgres).await.unwrap());

        if settings.environment == Environment::Local {
            postgres.do_migrations().await;
        }

        Self {
            estimator: FuelEstimator::new(settings.num_fuel_estimation_workers, postgres.clone()),
            live_fuel: LiveFuel::new(postgres),
            environment: settings.environment,
        }
    }

    pub async fn run(self) -> Result<()> {
        match self.environment {
            Environment::Local
            | Environment::Production
            | Environment::Development
            | Environment::OnPremise => {
                let mut set = JoinSet::new();

                set.spawn(self.estimator.run_continuous());
                set.spawn(self.live_fuel.run_continuous());

                // Unwrap only panics on empty set
                let out = set.join_next().await.unwrap()?;

                panic!("one task unexpectedly exited with output: {out:?}");
            }
            Environment::Test => {
                self.estimator.run_single().await?;
                self.live_fuel.run_single().await
            }
        }
    }
}
