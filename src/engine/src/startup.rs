use crate::{settings::Settings, Engine, SharedState, StepWrapper};
use orca_core::Environment;
use orca_statemachine::{Machine, Pending};
use postgres::PostgresAdapter;
use scraper::{BarentswatchSource, FiskeridirSource, Scraper, WrappedHttpClient};
use std::{path::PathBuf, sync::Arc};

pub struct App {
    pub shared_state: SharedState<PostgresAdapter>,
    pub transition_log: orca_statemachine::PostgresAdapter,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        if settings.environment == Environment::Local {
            postgres.do_migrations().await;
        }

        let file_downloader = fiskeridir_rs::FileDownloader::new(PathBuf::from("/home")).unwrap();
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
        let transition_log = orca_statemachine::PostgresAdapter::new(&settings.postgres)
            .await
            .unwrap();

        let benchmarks = settings.benchmarks();
        let haul_distributors = settings.haul_distributors();
        let trip_distancers = settings.trip_distancers();

        let shared_state = SharedState::new(
            settings.engine.clone(),
            postgres,
            scraper,
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
        let step = StepWrapper::initial(
            self.transition_log.clone(),
            self.shared_state,
            Pending::default(),
        );
        Machine::run(Engine::Pending(step), self.transition_log).await
    }
}
