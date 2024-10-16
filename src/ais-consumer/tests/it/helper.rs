use ais_consumer::{
    models::{AisPosition, AisStatic},
    settings::Settings,
    startup::App,
};
use futures::{Future, TryStreamExt};
use kyogre_core::{VerificationOutbound, POSTGRES_TEST_PORT};
use orca_core::{Environment, PsqlLogStatements, PsqlSettings, TestHelperBuilder};
use postgres::{PostgresAdapter, TestDb};
use std::panic;
use tokio_stream::wrappers::ReceiverStream;

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

            let settings = PsqlSettings {
                ip: db_handle.ip().to_string(),
                port: 5432,
                db_name,
                username: "postgres".to_string(),
                password: None,
                max_connections: 1,
                root_cert: None,
                log_statements: PsqlLogStatements::Disable,
                application_name: None,
            };

            let db = PostgresAdapter::new(&settings).await.unwrap();

            let commit_interval = std::time::Duration::from_millis(5);

            let app_settings = Settings {
                environment: Environment::Test,
                postgres: settings,
                commit_interval,
                broadcast_buffer_size: 10,
                oauth: None,
                api_address: None,
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
