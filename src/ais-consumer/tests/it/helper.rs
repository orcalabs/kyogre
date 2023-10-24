use std::{panic, sync::Once};

use ais_consumer::{
    models::{AisPosition, AisStatic},
    settings::Settings,
    startup::App,
};
use dockertest::{DockerTest, Source};
use futures::{Future, TryStreamExt};
use kyogre_core::VerificationOutbound;
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use tokio_stream::wrappers::ReceiverStream;
use tracing_subscriber::FmtSubscriber;

static TRACING: Once = Once::new();

static DATABASE_PASSWORD: &str = "test123";

pub struct TestHelper {
    pub ais_source: AisSource,
    pub db: TestDb,
    pub consumer_commit_interval: std::time::Duration,
    pub postgres_process_confirmation: tokio::sync::mpsc::Receiver<()>,
}

pub struct AisSource {
    out: tokio::sync::mpsc::Sender<Result<String, std::io::Error>>,
}

impl TestHelper {}

pub async fn test<T, Fut>(test_closure: T)
where
    T: FnOnce(TestHelper) -> Fut + panic::UnwindSafe + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    TRACING.call_once(|| {
        tracing::subscriber::set_global_default(
            FmtSubscriber::builder()
                .with_max_level(tracing::Level::INFO)
                .finish(),
        )
        .unwrap();
    });

    let mut docker_test = DockerTest::new().with_default_source(Source::DockerHub);

    let db_composition = postgres_composition(
        DATABASE_PASSWORD,
        "postgres",
        "ghcr.io/orcalabs/kyogre/test-postgres",
        "latest",
    )
    .set_log_options(None);

    docker_test.provide_container(db_composition);

    let db_name = random::<u32>().to_string();

    docker_test
        .run_async(|ops| async move {
            // Necessary evil to avoid unzipping shore map for each test
            std::env::set_var("APP_ENVIRONMENT", "TEST");
            let db_handle = ops.handle("postgres");

            let master_db_settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name: Some("template1".to_string()),
                username: "postgres".to_string(),
                password: DATABASE_PASSWORD.to_string(),
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Disable,
            };

            let master_db = PostgresAdapter::new(&master_db_settings).await.unwrap();
            let test_master_db = TestDb { db: master_db };
            test_master_db
                .create_test_database_from_template(&db_name)
                .await;

            let settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name: Some(db_name.to_string()),
                username: "postgres".to_string(),
                password: DATABASE_PASSWORD.to_string(),
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Disable,
            };

            let db = PostgresAdapter::new(&settings).await.unwrap();

            let commit_interval = std::time::Duration::from_millis(5);

            let app_settings = Settings {
                log_level: LogLevel::Debug,
                environment: Environment::Test,
                postgres: settings,
                commit_interval,
                broadcast_buffer_size: 10,
                oauth: None,
                api_address: None,
                honeycomb: None,
            };

            let test_db = TestDb { db };

            let (postgres_sender, postgres_recveiver) = tokio::sync::mpsc::channel(100);
            let app = App::build(app_settings).await;

            let (sender, recv) = tokio::sync::mpsc::channel(100);

            let receiver_stream = ReceiverStream::new(recv);
            let compat = tokio_util::compat::FuturesAsyncReadCompatExt::compat(
                receiver_stream.into_async_read(),
            );

            tokio::spawn(app.run_test(compat, postgres_sender));

            let helper = TestHelper {
                ais_source: AisSource { out: sender },
                db: test_db.clone(),
                consumer_commit_interval: commit_interval,
                postgres_process_confirmation: postgres_recveiver,
            };

            test_closure(helper).await;

            test_db.db.verify_database().await.unwrap();

            test_master_db.drop_db(&db_name).await;
        })
        .await;
}

impl AisSource {
    pub async fn send_position(&self, position: &AisPosition) {
        let string = serde_json::to_string(position).unwrap();
        self.send_string(string).await
    }

    pub async fn send_static(&self, static_message: &AisStatic) {
        let string = serde_json::to_string(static_message).unwrap();
        self.send_string(string).await
    }

    async fn send_string(&self, mut val: String) {
        val.push('\n');
        self.out.send(Ok(val)).await.unwrap();
    }
}
