use engine::*;
use futures::Future;
use kyogre_core::{VerificationOutbound, POSTGRES_TEST_PORT};
use orca_core::TestHelperBuilder;
use orca_core::{PsqlLogStatements, PsqlSettings};
use postgres::{PostgresAdapter, TestDb};
use std::panic;

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
            &self.db_settings,
        )
        .await
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
    TestHelperBuilder::default()
        .add_postgres(
            None,
            "ghcr.io/orcalabs/kyogre/test-postgres",
            None,
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

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();

            let db = TestDb {
                db: adapter.clone(),
            };

            let helper = TestHelper { db, db_settings };
            let builder = helper.builder().await;

            test(helper, builder).await;

            adapter.verify_database().await.unwrap();
        })
        .await;
}
