use crate::{
    current_position::CurrentPositionProcessor, FuelEstimator, LiveFuel, Result, Settings,
};
use orca_core::Environment;
use postgres::PostgresAdapter;
use std::sync::Arc;
use tokio::task::JoinSet;

#[derive(Clone)]
pub struct App {
    pub estimator: FuelEstimator,
    live_fuel: LiveFuel,
    current_position: CurrentPositionProcessor,
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
            live_fuel: LiveFuel::new(postgres.clone()),
            current_position: CurrentPositionProcessor::new(
                postgres,
                settings.current_positions_batch_size,
            ),
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

                let Self {
                    estimator,
                    live_fuel,
                    current_position,
                    environment: _,
                } = self;

                set.spawn(estimator.run_continuous());
                set.spawn(live_fuel.run_continuous());
                set.spawn(current_position.run_continuous());

                // Unwrap only panics on empty set
                let out = set.join_next().await.unwrap()?;

                panic!("one task unexpectedly exited with output: {out:?}");
            }
            Environment::Test => {
                let Self {
                    estimator,
                    live_fuel,
                    current_position,
                    environment: _,
                } = self;

                estimator.run_single(None).await?;
                live_fuel.run_single().await?;
                current_position.run_single().await?;

                Ok(())
            }
        }
    }
}
