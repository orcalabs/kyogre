use dockertest::{DockerTest, Source, StartPolicy, StaticManagementPolicy};
use engine::states::{Pending, Scrape, Sleep, Trips, TripsPrecision};
use engine::*;
use futures::Future;
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use orca_statemachine::{Machine, Schedule, Transition, TransitionLog};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::str::FromStr;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";

pub struct TestHelper {
    settings: Settings,
    pub db: TestDb,
}

impl TestHelper {
    pub fn adapter(&self) -> &PostgresAdapter {
        &self.db.db
    }

    pub fn enable_scrape(&mut self) {
        self.settings.engine = engine::Config {
            scrape_schedule: Schedule::Periodic(std::time::Duration::from_micros(0)),
        };
    }

    pub fn disable_scrape(&mut self) {
        self.settings.engine = engine::Config {
            scrape_schedule: Schedule::Disabled,
        };
    }
    pub async fn run_step(&self, state: EngineDiscriminants) -> EngineDiscriminants {
        let app = App::build(&self.settings).await;
        let (to, from) = match &state {
            EngineDiscriminants::Trips => {
                let step =
                    StepWrapper::initial(app.transition_log.clone(), app.shared_state, Trips);
                let engine = Engine::Trips(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::Pending => {
                let step = StepWrapper::initial(
                    app.transition_log.clone(),
                    app.shared_state,
                    Pending::default(),
                );
                let engine = Engine::Pending(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::Sleep => {
                let step = StepWrapper::initial(
                    app.transition_log.clone(),
                    app.shared_state,
                    Sleep::default(),
                );
                let engine = Engine::Sleep(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::Scrape => {
                let step =
                    StepWrapper::initial(app.transition_log.clone(), app.shared_state, Scrape);
                let engine = Engine::Scrape(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::TripsPrecision => {
                let step = StepWrapper::initial(
                    app.transition_log.clone(),
                    app.shared_state,
                    TripsPrecision,
                );
                let engine = Engine::TripsPrecision(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::Benchmark => {
                let step =
                    StepWrapper::initial(app.transition_log.clone(), app.shared_state, Benchmark);
                let engine = Engine::Benchmark(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::HaulDistribution => {
                let step = StepWrapper::initial(
                    app.transition_log.clone(),
                    app.shared_state,
                    HaulDistribution,
                );
                let engine = Engine::HaulDistribution(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::TripDistance => {
                let step = StepWrapper::initial(
                    app.transition_log.clone(),
                    app.shared_state,
                    TripDistance,
                );
                let engine = Engine::TripDistance(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
            EngineDiscriminants::UpdateDatabaseViews => {
                let step = StepWrapper::initial(
                    app.transition_log.clone(),
                    app.shared_state,
                    UpdateDatabaseViews,
                );
                let engine = Engine::UpdateDatabaseViews(step);
                let from = engine.current_state_name();
                let engine = engine.step().await;
                let to = engine.current_state_name();
                (to, from)
            }
        };
        let current = EngineDiscriminants::from_str(&to).unwrap();

        let transition = Transition {
            date: chrono::offset::Utc::now(),
            to,
            from,
        };
        app.transition_log
            .add_transition(&transition)
            .await
            .unwrap();

        current
    }
}

pub async fn test<T, Fut>(engine_config: engine::Config, test: T)
where
    T: FnOnce(TestHelper) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut docker_test = DockerTest::new().with_default_source(Source::DockerHub);
    TRACING.call_once(|| {
        tracing::subscriber::set_global_default(
            FmtSubscriber::builder()
                .with_max_level(tracing::Level::INFO)
                .finish(),
        )
        .unwrap();
    });

    let mut postgres = postgres_composition(
        DATABASE_PASSWORD,
        "postgres",
        "ghcr.io/orcalabs/kyogre/test-postgres",
        "latest",
    )
    .with_log_options(None)
    .with_start_policy(StartPolicy::Strict);

    let db_name = random::<u32>().to_string();

    postgres.static_container(StaticManagementPolicy::Dynamic);

    docker_test.add_composition(postgres);

    docker_test
        .run_async(|ops| async move {
            let db_handle = ops.handle("postgres");

            let mut db_settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name: Some("template1".to_string()),
                password: DATABASE_PASSWORD.to_string(),
                username: "postgres".to_string(),
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Enable,
            };

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();

            let test_db = TestDb { db: adapter };

            test_db.create_test_database_from_template(&db_name).await;

            db_settings.db_name = Some(db_name.clone());
            let settings = Settings {
                log_level: LogLevel::Debug,
                telemetry: None,
                postgres: db_settings.clone(),
                environment: Environment::Test,
                honeycomb: None,
                engine: engine_config,
                scraper: scraper::Config::default(),
            };

            let db = TestDb {
                db: PostgresAdapter::new(&db_settings).await.unwrap(),
            };

            test(TestHelper { settings, db }).await;

            test_db.drop_db(&db_name).await;
        })
        .await;
}
