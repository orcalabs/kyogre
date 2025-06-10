use postgres::PostgresAdapter;

use crate::{Migrator, settings::Settings, source::Source};

pub struct App {
    migrator: Migrator<PostgresAdapter, Source>,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let destination = PostgresAdapter::new(&settings.destination).await.unwrap();

        let source = Source::new(destination.clone(), &settings.source).await;

        let migrator = Migrator::new(
            settings.start_threshold,
            settings.end_threshold,
            chrono::Duration::from_std(settings.chunk_size).unwrap(),
            source,
            destination,
        );

        App { migrator }
    }

    pub async fn run(self) {
        self.migrator.run().await;
    }
}
