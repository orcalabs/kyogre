use super::test_client::ApiClient;
use dockertest::{DockerTest, Source, StaticManagementPolicy};
use futures::Future;
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;
use web_api::{
    settings::{ApiSettings, Settings},
    startup::App,
};

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";

pub struct TestHelper {
    pub app: ApiClient,
    pub db: TestDb,
}

impl TestHelper {
    pub fn handle(&self) -> &PostgresAdapter {
        &self.db.db
    }
    async fn spawn_app(db: PostgresAdapter, app: App) -> TestHelper {
        let address = format!("http://127.0.0.1:{}/v1.0", app.port());

        tokio::spawn(async { app.run().await.unwrap() });

        TestHelper {
            app: ApiClient::new(address),
            db: TestDb { db },
        }
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
            let api_settings = Settings {
                log_level: LogLevel::Debug,
                telemetry: None,
                api: ApiSettings {
                    ip: "127.0.0.1".to_string(),
                    port: 0,
                },
                postgres: db_settings.clone(),
                environment: Environment::Test,
                honeycomb: None,
            };

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
            let app = TestHelper::spawn_app(adapter, App::build(&api_settings).await).await;

            test(app).await;

            test_db.drop_db(&db_name).await;
        })
        .await;
}
