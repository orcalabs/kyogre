use std::sync::Arc;

use kyogre_core::FiskeridirVesselId;
use machine::StateMachine;
use orca_core::Environment;
use postgres::PostgresAdapter;
use scraper::{FiskeridirSource, Scraper};

use crate::{FisheryDiscriminants, FisheryEngine, SharedState, settings::Settings};

pub struct App {
    pub shared_state: SharedState,
    pub transition_log: machine::PostgresAdapter,
    pub single_state_run: Option<FisheryDiscriminants>,
    local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        if settings.environment == Environment::Local {
            postgres.do_migrations().await;
        }

        std::fs::create_dir_all(&settings.scraper.file_download_dir)
            .expect("failed to create download dir");
        let file_downloader =
            fiskeridir_rs::DataDownloader::new(settings.scraper.file_download_dir.clone());
        let api_downloader = fiskeridir_rs::ApiDownloader::new();

        let fiskeridir_source =
            FiskeridirSource::new(Box::new(postgres.clone()), file_downloader, api_downloader);

        let scraper = Scraper::new(
            settings.environment,
            settings.scraper.clone(),
            Arc::new(postgres.clone()),
            fiskeridir_source,
        );
        let trip_assemblers = settings.trip_assemblers();
        let transition_log = machine::PostgresAdapter::new(&settings.postgres)
            .await
            .unwrap();

        let benchmarks = settings.benchmarks();
        let trip_distancer = settings.trip_distancer();
        let trip_position_layers = settings.trip_position_layers();

        let postgres_arc = Arc::new(postgres.clone());
        let postgres = Box::new(postgres);

        let shared_state = SharedState::new(
            settings.num_trip_state_workers,
            settings.local_processing_vessels.clone(),
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
            postgres_arc,
            Some(Box::new(scraper)),
            trip_assemblers,
            benchmarks,
            trip_distancer,
            trip_position_layers,
            settings.fuel_estimation_mode,
        );

        App {
            transition_log,
            shared_state,
            single_state_run: settings.single_state_run,
            local_processing_vessels: settings.local_processing_vessels.clone(),
        }
    }

    pub async fn run(self) {
        if self.local_processing_vessels.is_some() {
            FisheryEngine::Trips(crate::Step::initial(
                crate::TripsState,
                self.shared_state,
                Box::new(self.transition_log),
            ))
            // Run Trips
            .run_single()
            .await
            // Run Benchmark
            .run_single()
            .await;
        } else if let Some(start_state) = self.single_state_run {
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
                FisheryDiscriminants::VerifyDatabase => {
                    let step = crate::Step::initial(
                        crate::VerifyDatabaseState,
                        self.shared_state,
                        Box::new(self.transition_log),
                    );
                    let engine = FisheryEngine::VerifyDatabase(step);
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

            engine.run().await;
        }
    }
}
