use duckdb_rs::{
    adapter::{CacheMode, CacheStorage, DuckdbSettings},
    api::Client,
    settings::Settings,
    startup::App,
};
use futures::Future;
use kyogre_core::{VerificationOutbound, POSTGRES_TEST_PORT};
use orca_core::{Environment, PsqlLogStatements, PsqlSettings, TestHelperBuilder};
use postgres::{PostgresAdapter, TestDb};
use std::panic;

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
    TestHelperBuilder::default()
        .add_postgres(
            "ghcr.io/orcalabs/kyogre/test-postgres",
            Some(POSTGRES_TEST_PORT),
        )
        .build()
        .run(move |ops, db_name| async move {
            let db_handle = ops.handle("postgres");

            let db_settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name,
                password: None,
                username: "postgres".to_string(),
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Enable,
                application_name: None,
            };

            let settings = Settings {
                port: 0,
                postgres: db_settings.clone(),
                environment: Environment::Test,
                duck_db: DuckdbSettings {
                    max_connections: 10,
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
        })
        .await;
}
