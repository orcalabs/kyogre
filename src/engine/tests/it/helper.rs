use dockertest::{DockerTest, Source, StartPolicy};
use engine::*;
use futures::Future;
use kyogre_core::{VerificationOutbound, KEEP_DB_ENV};
use orca_core::{compositions::postgres_composition, PsqlLogStatements, PsqlSettings};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;

static TRACING: Once = Once::new();

pub struct TestHelper {
    pub db: TestDb,
    db_settings: PsqlSettings,
}

impl TestHelper {
    pub async fn builder(&self) -> TestStateBuilder {
        let engine = engine(self.adapter().clone(), &self.db_settings).await;

        TestStateBuilder::new(
            Box::new(self.adapter().clone()),
            Box::new(self.adapter().clone()),
            engine,
        )
    }
    pub fn adapter(&self) -> &PostgresAdapter {
        &self.db.db
    }
}

pub async fn test<T, Fut>(test: T)
where
    T: FnOnce(TestHelper, TestStateBuilder) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut docker_test = DockerTest::new().with_default_source(Source::DockerHub);
    TRACING.call_once(|| {
        tracing::subscriber::set_global_default(
            FmtSubscriber::builder()
                .with_max_level(tracing::Level::ERROR)
                .finish(),
        )
        .unwrap();
    });

    let mut postgres = postgres_composition(
        "postgres",
        "ghcr.io/orcalabs/kyogre/test-postgres",
        "latest",
    )
    .set_log_options(None)
    .set_start_policy(StartPolicy::Strict);

    let mut keep_db = false;
    let db_name = if let Ok(v) = std::env::var(KEEP_DB_ENV) {
        if v == "true" {
            keep_db = true;
            "test".into()
        } else {
            random::<u32>().to_string()
        }
    } else {
        random::<u32>().to_string()
    };

    postgres.modify_port_map(5432, 5400);

    docker_test.provide_container(postgres);

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
            let mut test_db = TestDb { db: adapter };

            if keep_db {
                test_db.drop_db(&db_name).await;
                let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
                test_db = TestDb { db: adapter };
            }

            test_db.create_test_database_from_template(&db_name).await;

            db_settings.db_name = Some(db_name.clone());

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
            let db = TestDb {
                db: adapter.clone(),
            };

            let helper = TestHelper { db, db_settings };
            let builder = helper.builder().await;

            test(helper, builder).await;

            adapter.verify_database().await.unwrap();

            if !keep_db {
                test_db.drop_db(&db_name).await;
            }
        })
        .await;
}
