use std::panic;

use consumer::{
    models::{AisPosition, AisStatic},
    settings::Settings,
    startup::App,
};
use dockertest::{DockerTest, Source, StaticManagementPolicy};
use futures::{Future, TryStreamExt};
use lazy_static::{initialize, lazy_static};
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use tokio_stream::wrappers::ReceiverStream;
use tracing_subscriber::FmtSubscriber;

lazy_static! {
    static ref TRACING: () = tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish(),
    )
    .unwrap();
}

pub struct TestHelper {
    pub ais_source: AisSource,
    pub db: TestDb,
    pub consumer_commit_interval: std::time::Duration,
    pub consumer_cancellation: tokio::sync::mpsc::Sender<()>,
    pub postgres_cancellation: tokio::sync::mpsc::Sender<()>,
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
    initialize(&TRACING);
    let mut docker_test = DockerTest::new().with_default_source(Source::DockerHub);

    let database_password = "test123".to_string();

    let mut db_composition =
        postgres_composition(&database_password, "test-db", "postgres", "15.1-alpine")
            .with_log_options(None);
    db_composition.static_container(StaticManagementPolicy::Dynamic);

    docker_test.add_composition(db_composition);

    let db_name = random::<u32>().to_string();

    docker_test
        .run_async(|ops| async move {
            // Necessary evil to avoid unzipping shore map for each test
            std::env::set_var("APP_ENVIRONMENT", "TEST");
            let db_handle = ops.handle("test-db");

            let master_db_settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name: None,
                username: "postgres".to_string(),
                password: database_password.clone(),
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Disable,
            };

            let master_db = PostgresAdapter::new(&master_db_settings).await.unwrap();
            let test_master_db = TestDb { db: master_db };
            test_master_db.create_test_database(&db_name).await;

            let settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name: Some(db_name.to_string()),
                username: "postgres".to_string(),
                password: database_password,
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
            };

            let test_db = TestDb { db };
            test_db.do_migrations().await;

            let (postgres_cancellation, postgres_recv_cancel) = tokio::sync::mpsc::channel(1);
            let app = App::build(app_settings).await;

            let (sender, recv) = tokio::sync::mpsc::channel(100);

            let receiver_stream = ReceiverStream::new(recv);
            let compat = tokio_util::compat::FuturesAsyncReadCompatExt::compat(
                receiver_stream.into_async_read(),
            );

            let (consumer_cancellation, consumer_recv_cancel) = tokio::sync::mpsc::channel(1);

            tokio::spawn(app.run_test(compat, postgres_recv_cancel, consumer_recv_cancel));

            let helper = TestHelper {
                ais_source: AisSource { out: sender },
                db: test_db.clone(),
                consumer_commit_interval: commit_interval,
                consumer_cancellation,
                postgres_cancellation,
            };

            test_closure(helper).await;
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
