use super::test_client::ApiClient;
use chrono::{DateTime, Utc};
use dockertest::{DockerTest, Source, StaticManagementPolicy};
use fiskeridir_rs::{ErsDep, ErsPor};
use futures::Future;
use kyogre_core::{
    ScraperInboundPort, TripAssemblerId, TripAssemblerInboundPort, TripAssemblerOutboundPort,
};
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;
use trip_assembler::{ErsTripAssembler, LandingTripAssembler, TripAssembler};
use web_api::{
    settings::{ApiSettings, Settings},
    startup::App,
};

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";

pub struct TestHelper {
    pub app: ApiClient,
    pub db: TestDb,
    ers_assembler: ErsTripAssembler,
    landings_assembler: LandingTripAssembler,
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
            ers_assembler: ErsTripAssembler::default(),
            landings_assembler: LandingTripAssembler::default(),
        }
    }

    pub async fn generate_ers_trip(
        &self,
        fiskeridir_vessel_id: i64,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> kyogre_core::Trip {
        let mut departure = ErsDep::test_default(random(), Some(fiskeridir_vessel_id as u64));
        departure.departure_date = start.date_naive();
        departure.departure_time = start.time();
        let mut arrival = ErsPor::test_default(random(), Some(fiskeridir_vessel_id as u64), true);
        arrival.arrival_date = end.date_naive();
        arrival.arrival_time = end.time();
        self.db.db.add_ers_por(vec![arrival]).await.unwrap();
        self.db.db.add_ers_dep(vec![departure]).await.unwrap();

        self.add_trip(fiskeridir_vessel_id, start, end, TripAssemblerId::Ers)
            .await
    }

    pub async fn generate_landings_trip(
        &self,
        fiskeridir_vessel_id: i64,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> kyogre_core::Trip {
        let mut landing =
            fiskeridir_rs::Landing::test_default(random(), Some(fiskeridir_vessel_id));
        landing.landing_timestamp = *start;

        let mut landing2 =
            fiskeridir_rs::Landing::test_default(random(), Some(fiskeridir_vessel_id));
        landing2.landing_timestamp = *end;

        self.db
            .db
            .add_landings(vec![landing, landing2])
            .await
            .unwrap();

        self.add_trip(fiskeridir_vessel_id, start, end, TripAssemblerId::Landings)
            .await
    }

    async fn add_trip(
        &self,
        fiskeridir_vessel_id: i64,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
        assembler: TripAssemblerId,
    ) -> kyogre_core::Trip {
        let vessel = self
            .db
            .db
            .vessels()
            .await
            .unwrap()
            .into_iter()
            .find(|v| v.fiskeridir.id == fiskeridir_vessel_id)
            .unwrap();

        let assembled = match assembler {
            TripAssemblerId::Landings => self
                .landings_assembler
                .assemble(&self.db.db, &vessel, trip_assembler::State::NoPriorState)
                .await
                .unwrap()
                .unwrap(),
            TripAssemblerId::Ers => self
                .ers_assembler
                .assemble(&self.db.db, &vessel, trip_assembler::State::NoPriorState)
                .await
                .unwrap()
                .unwrap(),
        };

        self.db
            .db
            .add_trips(
                vessel.fiskeridir.id,
                assembled.new_trip_calculation_time,
                assembled.conflict_strategy,
                assembled.trips,
                assembler,
            )
            .await
            .unwrap();

        let trips = self.db.trips_of_vessel(vessel.fiskeridir.id).await;
        trips
            .into_iter()
            .find(|t| t.start() == *start && t.end() == *end)
            .unwrap()
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
