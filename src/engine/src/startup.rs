use crate::{settings::Settings, FisheryDiscriminants, FisheryEngine, SharedState};
use machine::StateMachine;
use meilisearch::MeilisearchAdapter;
use orca_core::Environment;
use postgres::PostgresAdapter;
use scraper::{BarentswatchSource, FiskeridirSource, Scraper, WrappedHttpClient};
use std::sync::Arc;
use tokio::select;
use tracing::{event, Level};

pub struct App {
    pub shared_state: SharedState,
    pub transition_log: machine::PostgresAdapter,
    pub single_state_run: Option<FisheryDiscriminants>,
    meilisearch: Option<MeilisearchAdapter<PostgresAdapter>>,
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

        let meilisearch = if let Some(s) = &settings.meilisearch {
            let meilisearch = MeilisearchAdapter::new(s, postgres.clone());
            if matches!(
                settings.environment,
                Environment::Local | Environment::Development
            ) {
                meilisearch.create_indexes().await.unwrap();
            }
            Some(meilisearch)
        } else {
            None
        };

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
            settings.environment,
            settings.scraper.clone(),
            Arc::new(postgres.clone()),
            fiskeridir_source,
            barentswatch_source,
        );
        let trip_assemblers = settings.trip_assemblers();
        let transition_log = machine::PostgresAdapter::new(&settings.postgres)
            .await
            .unwrap();

        let benchmarks = settings.benchmarks();
        let trip_distancer = settings.trip_distancer();
        let ml_models = settings.ml_models();
        let trip_position_layers = settings.trip_position_layers();

        let postgres = Box::new(postgres);

        let shared_state = SharedState::new(
            settings.num_workers,
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
            postgres.clone(),
            postgres.clone(),
            postgres.clone(),
            postgres,
            Some(Box::new(scraper)),
            trip_assemblers,
            benchmarks,
            trip_distancer,
            ml_models,
            trip_position_layers,
        );

        App {
            transition_log,
            shared_state,
            single_state_run: settings.single_state_run,
            meilisearch,
        }
    }

    pub async fn run(self) {
        if let Some(start_state) = self.single_state_run {
            match start_state {
                FisheryDiscriminants::Scrape => {
                    let step = crate::Step::initial(
                        crate::ScrapeState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::Scrape(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::Trips => {
                    let step = crate::Step::initial(
                        crate::TripsState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::Trips(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::Benchmark => {
                    let step = crate::Step::initial(
                        crate::BenchmarkState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::Benchmark(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::HaulDistribution => {
                    let step = crate::Step::initial(
                        crate::HaulDistributionState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::HaulDistribution(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::HaulWeather => {
                    let step = crate::Step::initial(
                        crate::HaulWeatherState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::HaulWeather(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::VerifyDatabase => {
                    let step = crate::Step::initial(
                        crate::VerifyDatabaseState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::VerifyDatabase(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::MLModels => {
                    let step = crate::Step::initial(
                        crate::MLModelsState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::MLModels(step);
                    engine.run_single().await;
                }
                FisheryDiscriminants::DailyWeather => {
                    let step = crate::Step::initial(
                        crate::DailyWeatherState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::DailyWeather(step);
                    engine.run_single().await;
                }
            };
        } else {
            let step = crate::Step::initial(
                crate::Pending::default(),
                self.shared_state,
                Box::new(self.transition_log),
            );
            let engine = FisheryEngine::Pending(step);

            if let Some(meilisearch) = self.meilisearch {
                let engine = tokio::spawn(engine.run());
                let meilisearch = tokio::spawn(meilisearch.run());

                select! {
                    _ = engine => {
                        event!(Level::ERROR, "engine exited unexpectedly");
                    },
                    _ = meilisearch => {
                        event!(Level::ERROR, "meilisearch exited unexpectedly");
                    },
                }
            } else {
                engine.run().await;
            }
        }
    }
}
