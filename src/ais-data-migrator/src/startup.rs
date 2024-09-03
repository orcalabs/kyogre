use crate::{settings::Settings, Migrator};
use postgres::PostgresAdapter;

pub struct App {
    migrator: Migrator<PostgresAdapter, PostgresAdapter>,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let source_adapter = PostgresAdapter::new(&settings.source).await.unwrap();
        let destination_adapter = PostgresAdapter::new(&settings.destination).await.unwrap();

        let migrator = Migrator::new(
            settings.start_threshold,
            settings.end_threshold,
            chrono::Duration::from_std(settings.chunk_size).unwrap(),
            source_adapter,
            destination_adapter,
        );

        App { migrator }
    }

    pub async fn run(self) {
        self.migrator.run().await;
    }
}
