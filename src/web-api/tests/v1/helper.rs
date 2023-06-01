use super::{barentswatch_helper::BarentswatchHelper, test_client::ApiClient};
use chrono::{DateTime, Datelike, Duration, Utc};
use dockertest::{DockerTest, Source, StaticManagementPolicy};
use fiskeridir_rs::{ErsDep, ErsPor, GearGroup, SpeciesGroup, VesselLengthGroup};
use futures::Future;
use kyogre_core::*;
use orca_core::{
    compositions::postgres_composition, Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::panic;
use std::sync::Once;
use strum::EnumCount;
use tokio::sync::OnceCell;
use tracing_subscriber::FmtSubscriber;
use trip_assembler::{ErsTripAssembler, LandingTripAssembler, TripAssembler};
use vessel_benchmark::*;
use web_api::{
    duckdb::{CacheMode, CacheStorage, DuckdbAdapter, DuckdbSettings},
    routes::v1::haul,
    settings::{ApiSettings, Settings, BW_PROFILES_URL},
    startup::App,
};

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";
static BARENTSWATCH_HELPER: OnceCell<BarentswatchHelper> = OnceCell::const_new();

pub struct TestHelper {
    pub app: ApiClient,
    pub db: TestDb,
    pub bw_helper: &'static BarentswatchHelper,
    duck_db: Option<DuckdbAdapter>,
    ers_assembler: ErsTripAssembler,
    landings_assembler: LandingTripAssembler,
    weight_per_hour: WeightPerHour,
    ers_message_number: u32,
}

impl TestHelper {
    pub fn adapter(&self) -> &PostgresAdapter {
        &self.db.db
    }
    async fn spawn_app(
        db: PostgresAdapter,
        app: App,
        bw_helper: &'static BarentswatchHelper,
    ) -> TestHelper {
        let address = format!("http://127.0.0.1:{}/v1.0", app.port());

        let duck_db = app.duck_db.clone();

        tokio::spawn(async { app.run().await.unwrap() });

        TestHelper {
            app: ApiClient::new(address),
            db: TestDb { db },
            bw_helper,
            ers_assembler: ErsTripAssembler::default(),
            landings_assembler: LandingTripAssembler::default(),
            ers_message_number: 1,
            weight_per_hour: WeightPerHour::default(),
            duck_db,
        }
    }
    pub async fn do_benchmarks(&self) {
        self.weight_per_hour
            .produce_and_store_benchmarks(&self.db.db, &self.db.db)
            .await
            .unwrap();
    }
    pub fn refresh_cache(&self) {
        self.duck_db
            .as_ref()
            .unwrap()
            .refresh_hauls_cache_impl()
            .unwrap()
    }

    pub async fn add_precision_to_trip(&self, vessel: &Vessel, trip: &TripDetailed) -> DateRange {
        let ports = self.db.db.ports_of_trip(trip.trip_id).await.unwrap();

        let precision_start = trip.period.start() - Duration::seconds(1);
        let precision_end = trip.period.end() + Duration::seconds(1);
        // We need atleast a single point within trip to enable precision, the
        // ports precision for the ers assempler only operates on position outside the
        // trip period.
        let trip_point = trip.period.end() - Duration::seconds(1);

        assert!(trip_point < trip.period.end() && trip_point > trip.period.start());
        assert!(precision_start < trip.period.end());
        assert!(precision_end > trip.period.start());

        let start_port_cords = ports.start.unwrap().coordinates.unwrap();
        let end_port_cords = ports.end.unwrap().coordinates.unwrap();
        self.db
            .generate_ais_position_with_coordinates(
                vessel.mmsi().unwrap(),
                precision_start,
                start_port_cords.latitude,
                start_port_cords.longitude,
            )
            .await;
        self.db
            .generate_ais_position(vessel.mmsi().unwrap(), trip_point)
            .await;
        self.db
            .generate_ais_position_with_coordinates(
                vessel.mmsi().unwrap(),
                precision_end,
                end_port_cords.latitude,
                end_port_cords.longitude,
            )
            .await;

        let mut updates = self
            .ers_assembler
            .calculate_precision(vessel, &self.db.db, vec![Trip::from(trip.clone())])
            .await
            .unwrap();
        assert_eq!(1, updates.len());

        self.db
            .db
            .update_trip_precisions(updates.clone())
            .await
            .unwrap();
        self.db.db.update_database_views().await.unwrap();

        match updates.pop().unwrap().outcome {
            PrecisionOutcome::Success {
                new_period: new_range,
                start_precision: _,
                end_precision: _,
            } => new_range,
            PrecisionOutcome::Failed => panic!("failed to compute trip precision"),
        }
    }

    pub async fn generate_ers_trip(
        &mut self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> TripDetailed {
        let departure = ErsDep::test_default(
            random(),
            vessel_id.0 as u64,
            *start,
            self.ers_message_number,
        );
        self.ers_message_number += 1;
        let arrival =
            ErsPor::test_default(random(), vessel_id.0 as u64, *end, self.ers_message_number);
        self.ers_message_number += 1;
        self.db.db.add_ers_dep(vec![departure]).await.unwrap();
        self.db.db.add_ers_por(vec![arrival]).await.unwrap();

        self.add_trip(vessel_id, start, end, TripAssemblerId::Ers)
            .await
    }

    pub async fn generate_ers_trip_with_messages(
        &self,
        vessel_id: FiskeridirVesselId,
        departure: ErsDep,
        arrival: ErsPor,
    ) -> TripDetailed {
        let start = departure.departure_timestamp;
        let end = arrival.arrival_timestamp;
        self.db.db.add_ers_dep(vec![departure]).await.unwrap();
        self.db.db.add_ers_por(vec![arrival]).await.unwrap();

        self.add_trip(vessel_id, &start, &end, TripAssemblerId::Ers)
            .await
    }

    pub async fn generate_landings_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> TripDetailed {
        let mut landing = fiskeridir_rs::Landing::test_default(random(), Some(vessel_id.0));
        landing.landing_timestamp = *start;

        let mut landing2 = fiskeridir_rs::Landing::test_default(random(), Some(vessel_id.0));
        landing2.landing_timestamp = *end;

        let year = landing.landing_timestamp.year() as u32;

        self.db
            .db
            .add_landings(vec![landing, landing2], year)
            .await
            .unwrap();

        self.add_trip(vessel_id, start, end, TripAssemblerId::Landings)
            .await
    }

    async fn add_trip(
        &self,
        vessel_id: FiskeridirVesselId,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
        assembler: TripAssemblerId,
    ) -> TripDetailed {
        let adapter = self.adapter();

        match assembler {
            TripAssemblerId::Landings => self
                .landings_assembler
                .produce_and_store_trips(adapter)
                .await
                .unwrap(),
            TripAssemblerId::Ers => self
                .ers_assembler
                .produce_and_store_trips(adapter)
                .await
                .unwrap(),
        };

        self.db.db.update_database_views().await.unwrap();

        let trips = self.db.all_detailed_trips_of_vessels(vessel_id).await;

        trips
            .into_iter()
            .find(|t| {
                t.period.start().timestamp() == start.timestamp()
                    && t.period.end().timestamp() == end.timestamp()
                    && t.assembler_id == assembler
            })
            .unwrap()
    }
}

pub async fn test<T, Fut>(test: T)
where
    T: FnOnce(TestHelper) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    test_impl(test, None).await;
}

pub async fn test_with_cache<T, Fut>(test: T)
where
    T: FnOnce(TestHelper) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    test_impl(
        test,
        Some(DuckdbSettings {
            max_connections: 1,
            mode: CacheMode::ReturnError,
            storage: CacheStorage::Memory,
        }),
    )
    .await;
}

async fn test_impl<T, Fut>(test: T, duck_db: Option<DuckdbSettings>)
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

            let bw_helper = BARENTSWATCH_HELPER
                .get_or_init(|| async { BarentswatchHelper::new().await })
                .await;
            let bw_address = bw_helper.address();

            db_settings.db_name = Some(db_name.clone());
            let api_settings = Settings {
                log_level: LogLevel::Debug,
                telemetry: None,
                api: ApiSettings {
                    ip: "127.0.0.1".to_string(),
                    port: 0,
                    num_workers: Some(1),
                },
                postgres: db_settings.clone(),
                environment: Environment::Test,
                honeycomb: None,
                bw_jwks_url: Some(format!("{bw_address}/jwks")),
                bw_profiles_url: Some(format!("{bw_address}/profiles")),
                duck_db,
            };

            let _ = BW_PROFILES_URL.set(api_settings.bw_profiles_url.clone().unwrap());

            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
            let app =
                TestHelper::spawn_app(adapter, App::build(&api_settings).await, bw_helper).await;

            test(app).await;

            test_db.drop_db(&db_name).await;
        })
        .await;
}

pub fn sum_area(matrix: &[u64], x0: usize, y0: usize, x1: usize, y1: usize, width: usize) -> u64 {
    let mut sum = matrix[y1 * width + x1];
    if x0 > 0 {
        sum -= matrix[y1 * width + x0 - 1];
    }

    if y0 > 0 {
        sum -= matrix[(y0 - 1) * width + x1];
    }

    if x0 > 0 && y0 > 0 {
        sum += matrix[(y0 - 1) * width + x0 - 1];
    }

    sum
}

pub fn assert_matrix_content(
    matrix: &haul::HaulsMatrix,
    active_filter: ActiveHaulsFilter,
    expected_total: u64,
) {
    let y_dimension = match active_filter {
        ActiveHaulsFilter::Date => date_feature_matrix_size(),
        ActiveHaulsFilter::GearGroup => GearGroup::COUNT,
        ActiveHaulsFilter::SpeciesGroup => SpeciesGroup::COUNT,
        ActiveHaulsFilter::VesselLength => VesselLengthGroup::COUNT,
        ActiveHaulsFilter::CatchLocation => NUM_CATCH_LOCATIONS,
    };

    assert_eq!(
        (date_feature_matrix_size()
            * if matches!(active_filter, ActiveHaulsFilter::Date) {
                NUM_CATCH_LOCATIONS
            } else {
                y_dimension
            }),
        matrix.dates.len()
    );
    assert_eq!(
        (GearGroup::COUNT
            * if matches!(active_filter, ActiveHaulsFilter::GearGroup) {
                NUM_CATCH_LOCATIONS
            } else {
                y_dimension
            }),
        matrix.gear_group.len()
    );
    assert_eq!(
        (VesselLengthGroup::COUNT
            * if matches!(active_filter, ActiveHaulsFilter::VesselLength) {
                NUM_CATCH_LOCATIONS
            } else {
                y_dimension
            }),
        matrix.length_group.len()
    );

    assert_eq!(
        (SpeciesGroup::COUNT
            * if matches!(active_filter, ActiveHaulsFilter::SpeciesGroup) {
                NUM_CATCH_LOCATIONS
            } else {
                y_dimension
            }),
        matrix.species_group.len()
    );

    if !matches!(active_filter, ActiveHaulsFilter::Date) {
        assert_eq!(expected_total, matrix.dates[matrix.dates.len() - 1]);
    }
    if !matches!(active_filter, ActiveHaulsFilter::GearGroup) {
        assert_eq!(
            expected_total,
            matrix.gear_group[matrix.gear_group.len() - 1]
        );
    }
    if !matches!(active_filter, ActiveHaulsFilter::VesselLength) {
        assert_eq!(
            expected_total,
            matrix.length_group[matrix.length_group.len() - 1]
        );
    }
    if !matches!(active_filter, ActiveHaulsFilter::SpeciesGroup) {
        assert_eq!(
            expected_total,
            matrix.species_group[matrix.species_group.len() - 1]
        );
    }
}
