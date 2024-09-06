use dockertest::{DockerTest, Source};
use duckdb_rs::{
    adapter::{CacheMode, CacheStorage, DuckdbSettings},
    api::Client,
    settings::Settings,
    startup::App,
};
use futures::Future;
use kyogre_core::VerificationOutbound;
use orca_core::{compositions::postgres_composition, Environment, PsqlLogStatements, PsqlSettings};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;

static TRACING: Once = Once::new();

pub struct TestHelper {
    pub db: TestDb,
    pub cache: Client,
}

impl TestHelper {
    pub fn adapter(&self) -> &PostgresAdapter {
        &self.db.db
    }
}

pub async fn test<T, Fut>(test: T)
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

    let composition = postgres_composition(
        "postgres",
        "ghcr.io/orcalabs/kyogre/test-postgres",
        "latest",
    )
    .set_log_options(None);

    docker_test.provide_container(composition);

    let db_name = random::<u32>().to_string();
    docker_test
        .run_async(|ops| async move {
            let db_handle = ops.handle("postgres");

            let mut db_settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name: Some("template1".to_string()),
                password: None,
                username: "postgres".to_string(),
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Enable,
                application_name: None,
            };

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
            let test_db = TestDb { db: adapter };

            test_db.create_test_database_from_template(&db_name).await;

            db_settings.db_name = Some(db_name.clone());
            let settings = Settings {
                port: 0,
                postgres: db_settings.clone(),
                environment: Environment::Test,
                duck_db: DuckdbSettings {
                    max_connections: 1,
                    cache_mode: CacheMode::ReturnError,
                    storage: CacheStorage::Memory,
                    refresh_interval: std::time::Duration::from_secs(10000000),
                },
            };

            let app = App::build(&settings).await;
            let port = app.port();

            tokio::spawn(app.run());
            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();

            let helper = TestHelper {
                db: TestDb {
                    db: adapter.clone(),
                },
                cache: Client::new("[::]", port).await.unwrap(),
            };

            test(helper).await;

            adapter.verify_database().await.unwrap();

            test_db.drop_db(&db_name).await;
        })
        .await;
}
