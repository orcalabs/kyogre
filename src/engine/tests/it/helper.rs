use dockertest::{DockerTest, Source, StartPolicy, StaticManagementPolicy};
use engine::*;
use futures::Future;
use kyogre_core::VerificationOutbound;
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";

pub struct TestHelper {
    pub db: TestDb,
}

impl TestHelper {
    pub fn adapter(&self) -> &PostgresAdapter {
        &self.db.db
    }
}

pub async fn test<T, Fut>(test: T)
where
    T: FnOnce(TestHelper, App) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
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
                scraper: scraper::Config::default(),
                single_state_run: None,
            };

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
            let db = TestDb {
                db: adapter.clone(),
            };

            test(TestHelper { db }, App::build(&settings).await).await;

            adapter.verify_database().await.unwrap();

            test_db.drop_db(&db_name).await;
        })
        .await;
}
