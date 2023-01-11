use crate::{settings::Settings, Migrator};
use kyogre_core::{AisMigratorDestination, AisMigratorSource};
use leviathan_postgres::LeviathanPostgresAdapter;
use postgres::PostgresAdapter;

pub struct App {
    migrator: Migrator,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let source_adapter = Box::new(
            LeviathanPostgresAdapter::new(&settings.source)
                .await
                .unwrap(),
        );
        let destination_adapter =
            Box::new(PostgresAdapter::new(&settings.destination).await.unwrap());

        let migrator = Migrator::new(
            settings.source_start_threshold,
            settings.destination_end_threshold,
            chrono::Duration::from_std(settings.chunk_size).unwrap(),
            source_adapter as Box<dyn AisMigratorSource>,
            destination_adapter as Box<dyn AisMigratorDestination>,
        );

        App { migrator }
    }

    pub async fn run(self) {
        self.migrator.run().await;
    }
}
