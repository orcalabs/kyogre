use std::panic;

use dockertest::{DockerTest, Source, StaticManagementPolicy};
use futures::Future;
use kyogre_core::VerificationOutbound;
use orca_core::{compositions::postgres_composition, PsqlLogStatements, PsqlSettings};
use postgres::{PostgresAdapter, TestDb};
use rand::random;

static DATABASE_PASSWORD: &str = "test123";

pub struct TestHelper {
    pub db: TestDb,
}

pub async fn test<T, Fut>(test: T)
where
    T: FnOnce(TestHelper) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut docker_test = DockerTest::new().with_default_source(Source::DockerHub);

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

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
            let helper = TestHelper {
                db: TestDb {
                    db: adapter.clone(),
                },
            };

            test(helper).await;

            adapter.verify_database().await.unwrap();

            test_db.drop_db(&db_name).await;
        })
        .await;
}
