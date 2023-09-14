use crate::{settings::Settings, FisheryEngine, SharedState};
use machine::StateMachine;
use orca_core::Environment;
use postgres::PostgresAdapter;
use scraper::{BarentswatchSource, FiskeridirSource, Scraper, WrappedHttpClient};
use std::sync::Arc;

pub struct App {
    pub shared_state: SharedState,
    pub transition_log: machine::PostgresAdapter,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        if matches!(
            settings.environment,
            Environment::Local | Environment::Development
        ) {
            postgres.do_migrations().await;
        }

        std::fs::create_dir_all(&settings.scraper.file_download_dir)
            .expect("failed to create download dir");
        let file_downloader =
            fiskeridir_rs::FileDownloader::new(settings.scraper.file_download_dir.clone()).unwrap();
        let api_downloader = fiskeridir_rs::ApiDownloader::new().unwrap();

        let fiskeridir_source =
            FiskeridirSource::new(Box::new(postgres.clone()), file_downloader, api_downloader);

        let http_client = Arc::new(WrappedHttpClient::new().unwrap());

        let barentswatch_source = BarentswatchSource::new(http_client);

        let scraper = Scraper::new(
            settings.scraper.clone(),
            Box::new(postgres.clone()),
            fiskeridir_source,
            barentswatch_source,
        );
        let trip_assemblers = settings.trip_assemblers();
        let transition_log = machine::PostgresAdapter::new(&settings.postgres)
            .await
            .unwrap();

        let benchmarks = settings.benchmarks();
        let haul_distributors = settings.haul_distributors();
        let trip_distancers = settings.trip_distancers();

        let postgres = Box::new(postgres);

        let shared_state = SharedState::new(
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres,
            Some(Box::new(scraper)),
            trip_assemblers,
            benchmarks,
            haul_distributors,
            trip_distancers,
        );

        App {
            transition_log,
            shared_state,
        }
    }

    pub async fn run(self) {
        let step = crate::Step::initial(
            crate::Pending::default(),
            self.shared_state,
            Box::new(self.transition_log),
        );
        let engine = FisheryEngine::Pending(step);
        engine.run().await;
    }
}
