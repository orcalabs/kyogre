use dockertest::{DockerTest, Source, StaticManagementPolicy};
use duckdb_rs::{
    adapter::{CacheMode, CacheStorage, DuckdbSettings},
    api::Client,
    settings::Settings,
    startup::App,
};
use futures::Future;
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::sync::Once;
use std::{net::Ipv4Addr, panic};
use tracing_subscriber::FmtSubscriber;

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";

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

    let mut composition = postgres_composition(
        DATABASE_PASSWORD,
        "postgres",
        "ghcr.io/orcalabs/kyogre/test-postgres",
        "latest",
    )
    .with_log_options(None);

    composition.static_container(StaticManagementPolicy::Dynamic);

    docker_test.add_composition(composition);

    // let mut duckdb = Composition::with_repository("ghcr.io/orcalabs/kyogre/duckdb")
    //     .with_start_policy(StartPolicy::Strict)
    //     .with_container_name("duckdb")
    //     .with_wait_for(Box::new(MessageWait {
    //         message: "starting duckdb...".to_string(),
    //         source: MessageSource::Stdout,
    //         timeout: 60,
    //     }))
    //     .with_log_options(None);

    // duckdb.static_container(StaticManagementPolicy::Dynamic);
    // duckdb.inject_container_name(postgres.handle(), "KYOGRE_DUCKDB__POSTGRES__IP");
    // duckdb.env("APP_ENVIRONMENT", "test");
    // duckdb.env("KYOGRE_DUCKDB__POSTGRES__PORT", 5432);
    // duckdb.env("KYOGRE_DUCKDB__POSTGRES__DB_NAME", db_name.clone());
    // duckdb.env("KYOGRE_DUCKDB__POSTGRES__USERNAME", "postgres");
    // duckdb.env("KYOGRE_DUCKDB__POSTGRES__PASSWORD", DATABASE_PASSWORD);
    // duckdb.env("KYOGRE_DUCKDB__POSTGRES__MAX_CONNECTIONS", 1);
    // duckdb.env("KYOGRE_DUCKDB__POSTGRES__LOG_STATEMENTS", "Enable");
    // duckdb.env("KYOGRE_DUCKDB__DUCK_DB__MAX_CONNECTIONS", 1);
    // duckdb.env("KYOGRE_DUCKDB__DUCK_DB__MODE", "ReturnError");
    // duckdb.env("KYOGRE_DUCKDB__DUCK_DB__STORAGE", "Memory");
    // duckdb.env("KYOGRE_DUCKDB__DUCK_DB__STORAGE", "Memory");
    // duckdb.env("KYOGRE_DUCKDB__LOG_LEVEL", "Info");
    // duckdb.env("KYOGRE_DUCKDB__ENVIRONMENT", "TEST");
    // duckdb.env("KYOGRE_DUCKDB__PORT", 5000);

    let db_name = random::<u32>().to_string();
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
                port: 0,
                postgres: db_settings.clone(),
                environment: Environment::Test,
                honeycomb: None,
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
                db: TestDb { db: adapter },
                cache: Client::new(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST), port)
                    .await
                    .unwrap(),
            };

            test(helper).await;

            test_db.drop_db(&db_name).await;
        })
        .await;
}
