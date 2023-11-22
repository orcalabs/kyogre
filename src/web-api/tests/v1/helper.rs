use super::{barentswatch_helper::BarentswatchHelper, test_client::ApiClient};
use dockertest::{DockerTest, Source};
use duckdb_rs::{adapter, CacheStorage};
use engine::*;
use futures::Future;
use kyogre_core::*;
use meilisearch::MeilisearchAdapter;
use orca_core::{
    compositions::{meilisearch_composition, postgres_composition},
    Environment, LogLevel, PsqlLogStatements, PsqlSettings,
};
use postgres::{PostgresAdapter, TestDb};
use rand::random;
use std::ops::AddAssign;
use std::sync::Once;
use std::{ops::SubAssign, panic};
use strum::IntoEnumIterator;
use tokio::sync::OnceCell;
use tracing_subscriber::FmtSubscriber;
use web_api::{
    routes::v1::{haul, landing},
    settings::{ApiSettings, Duckdb, Settings, BW_PROFILES_URL},
    startup::App,
};

static TRACING: Once = Once::new();
static DATABASE_PASSWORD: &str = "test123";
static BARENTSWATCH_HELPER: OnceCell<BarentswatchHelper> = OnceCell::const_new();

//               Lon  Lat
pub const CL_00_05: (f64, f64) = (13.5, 67.125);
pub const CL_01_01: (f64, f64) = (41., 67.5);
pub const CL_01_03: (f64, f64) = (43.5, 67.5);
pub const CL_01_04: (f64, f64) = (47.5, 67.5);

pub struct TestHelper {
    pub app: ApiClient,
    pub db: TestDb,
    pub bw_helper: &'static BarentswatchHelper,
    duck_db: Option<duckdb_rs::Client>,
    db_settings: PsqlSettings,
    meilisearch: Option<MeilisearchAdapter<PostgresAdapter>>,
}

impl TestHelper {
    async fn builder(&self) -> TestStateBuilder {
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
    async fn spawn_app(
        db: PostgresAdapter,
        db_settings: PsqlSettings,
        app: App,
        bw_helper: &'static BarentswatchHelper,
        duck_db: Option<duckdb_rs::Client>,
        meilisearch: Option<MeilisearchAdapter<PostgresAdapter>>,
    ) -> TestHelper {
        let address = format!("http://127.0.0.1:{}/v1.0", app.port());

        tokio::spawn(async { app.run().await.unwrap() });

        TestHelper {
            app: ApiClient::new(address),
            db: TestDb { db },
            bw_helper,
            duck_db,
            db_settings,
            meilisearch,
        }
    }
    pub async fn refresh_matrix_cache(&self) {
        if let Some(duck_db) = self.duck_db.as_ref() {
            duck_db.refresh().await.unwrap();
        }
    }
    pub async fn refresh_cache(&self) {
        if let Some(meilisearch) = self.meilisearch.as_ref() {
            meilisearch.create_indexes().await.unwrap();
            meilisearch.refresh().await.unwrap();
        }
    }
}

pub async fn test<T, Fut>(test: T)
where
    T: FnOnce(TestHelper, TestStateBuilder) -> Fut + panic::UnwindSafe + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    test_impl(test, CacheMode::NoCache).await;
}

pub async fn test_with_matrix_cache<T, Fut>(test: T)
where
    T: FnOnce(TestHelper, TestStateBuilder) -> Fut
        + panic::UnwindSafe
        + Send
        + Sync
        + Clone
        + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    test_impl(test.clone(), CacheMode::NoCache).await;
    test_impl(test, CacheMode::MatrixCache).await;
}

pub async fn test_with_cache<T, Fut>(test: T)
where
    T: FnOnce(TestHelper, TestStateBuilder) -> Fut
        + panic::UnwindSafe
        + Send
        + Sync
        + Clone
        + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    test_impl(test.clone(), CacheMode::NoCache).await;
    test_impl(test, CacheMode::Meilisearch).await;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheMode {
    NoCache,
    MatrixCache,
    Meilisearch,
}

async fn test_impl<T, Fut>(test: T, cache_mode: CacheMode)
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

    let mut postgres = postgres_composition(
        DATABASE_PASSWORD,
        "postgres",
        "ghcr.io/orcalabs/kyogre/test-postgres",
        "latest",
    )
    .set_log_options(None);

    postgres.modify_port_map(5432, 5400);
    docker_test.provide_container(postgres);

    if cache_mode == CacheMode::Meilisearch {
        let mut meilisearch =
            meilisearch_composition("test123", "meilisearch", "getmeili/meilisearch", "v1.4.2")
                .set_log_options(None);
        meilisearch.modify_port_map(7700, 7500);
        docker_test.provide_container(meilisearch);
    }

    std::env::set_var("APP_ENVIRONMENT", "TEST");

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
            let mut test_db = TestDb { db: adapter };

            if keep_db {
                test_db.drop_db(&db_name).await;
                let adapter = PostgresAdapter::new(&db_settings).await.unwrap();
                test_db = TestDb { db: adapter };
            }

            test_db.create_test_database_from_template(&db_name).await;

            let bw_helper = BARENTSWATCH_HELPER
                .get_or_init(|| async { BarentswatchHelper::new().await })
                .await;
            let bw_address = bw_helper.address();

            db_settings.db_name = Some(db_name.clone());
            let adapter = PostgresAdapter::new(&db_settings).await.unwrap();

            let (duck_db_api, duck_db_client) = if cache_mode == CacheMode::MatrixCache {
                let duckdb_app = duckdb_rs::App::build(&duckdb_rs::Settings {
                    log_level: LogLevel::Debug,
                    telemetry: None,
                    postgres: db_settings.clone(),
                    environment: Environment::Test,
                    honeycomb: None,
                    duck_db: duckdb_rs::adapter::DuckdbSettings {
                        max_connections: 1,
                        cache_mode: adapter::CacheMode::ReturnError,
                        storage: CacheStorage::Memory,
                        refresh_interval: std::time::Duration::from_secs(100000),
                    },
                    port: 0,
                })
                .await;
                let port = duckdb_app.port();
                let ip = "127.0.0.1".to_string();
                tokio::spawn(duckdb_app.run());

                (
                    Some(Duckdb {
                        ip: ip.clone(),
                        port,
                    }),
                    Some(duckdb_rs::Client::new(ip, port).await.unwrap()),
                )
            } else {
                (None, None)
            };

            let (meilisearch, m_settings) = if cache_mode == CacheMode::Meilisearch {
                let handle = ops.handle("meilisearch");
                let settings = meilisearch::Settings {
                    host: format!("http://{}:7700", handle.ip()),
                    api_key: "test123".to_string(),
                    index_suffix: Some(db_name.clone()),
                };
                let meilisearch = MeilisearchAdapter::new(&settings, adapter.clone());
                if keep_db {
                    meilisearch.cleanup().await.unwrap();
                }
                (Some(meilisearch), Some(settings))
            } else {
                (None, None)
            };

            let api_settings = Settings {
                log_level: LogLevel::Debug,
                telemetry: None,
                api: ApiSettings {
                    ip: "127.0.0.1".to_string(),
                    port: 0,
                    num_workers: Some(1),
                },
                postgres: db_settings.clone(),
                meilisearch: m_settings,
                environment: Environment::Test,
                honeycomb: None,
                bw_jwks_url: Some(format!("{bw_address}/jwks")),
                bw_profiles_url: Some(format!("{bw_address}/profiles")),
                duck_db_api,
                auth0: None,
            };

            let _ = BW_PROFILES_URL.set(api_settings.bw_profiles_url.clone().unwrap());

            let app = TestHelper::spawn_app(
                adapter.clone(),
                db_settings,
                App::build(&api_settings).await,
                bw_helper,
                duck_db_client,
                meilisearch.clone(),
            )
            .await;

            dbg!(cache_mode);

            let builder = app.builder().await;
            test(app, builder).await;

            adapter.verify_database().await.unwrap();

            if !keep_db {
                test_db.drop_db(&db_name).await;
                if cache_mode == CacheMode::Meilisearch {
                    meilisearch.unwrap().cleanup().await.unwrap();
                }
            }
        })
        .await;
}

pub fn sum_area<T: SubAssign + AddAssign + Copy>(
    matrix: &[T],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    width: usize,
) -> T {
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

pub fn assert_landing_matrix_content(
    matrix: &landing::LandingMatrix,
    active_filter: ActiveLandingFilter,
    expected_total: u64,
    specific_totals: Vec<(LandingMatrixes, u64)>,
) {
    for m in LandingMatrixes::iter() {
        let y_dimension_size = if active_filter == m {
            NUM_CATCH_LOCATIONS
        } else {
            LandingMatrixes::from(active_filter).size()
        };

        let x_dimension_size = m.size();
        let (matrix_len, matrix_total) = match m {
            LandingMatrixes::Date => (matrix.dates.len(), matrix.dates[matrix.dates.len() - 1]),
            LandingMatrixes::GearGroup => (
                matrix.gear_group.len(),
                matrix.gear_group[matrix.gear_group.len() - 1],
            ),
            LandingMatrixes::SpeciesGroup => (
                matrix.species_group.len(),
                matrix.species_group[matrix.species_group.len() - 1],
            ),
            LandingMatrixes::VesselLength => (
                matrix.length_group.len(),
                matrix.length_group[matrix.length_group.len() - 1],
            ),
        };

        assert_eq!(matrix_len, x_dimension_size * y_dimension_size, "{m}");

        if let Some(specific) = specific_totals.iter().find(|v| v.0 == m) {
            assert_eq!(specific.1, matrix_total, "{m}");
        } else {
            assert_eq!(expected_total, matrix_total, "{m}");
        }
    }
}
pub fn assert_haul_matrix_content(
    matrix: &haul::HaulsMatrix,
    active_filter: ActiveHaulsFilter,
    expected_total: u64,
    specific_totals: Vec<(HaulMatrixes, u64)>,
) {
    for m in HaulMatrixes::iter() {
        let y_dimension_size = if active_filter == m {
            NUM_CATCH_LOCATIONS
        } else {
            HaulMatrixes::from(active_filter).size()
        };

        let x_dimension_size = m.size();
        let (matrix_len, matrix_total) = match m {
            HaulMatrixes::Date => (matrix.dates.len(), matrix.dates[matrix.dates.len() - 1]),
            HaulMatrixes::GearGroup => (
                matrix.gear_group.len(),
                matrix.gear_group[matrix.gear_group.len() - 1],
            ),
            HaulMatrixes::SpeciesGroup => (
                matrix.species_group.len(),
                matrix.species_group[matrix.species_group.len() - 1],
            ),
            HaulMatrixes::VesselLength => (
                matrix.length_group.len(),
                matrix.length_group[matrix.length_group.len() - 1],
            ),
        };

        assert_eq!(matrix_len, x_dimension_size * y_dimension_size, "{m}");

        if let Some(specific) = specific_totals.iter().find(|v| v.0 == m) {
            assert_eq!(specific.1, matrix_total, "{m}");
        } else {
            assert_eq!(expected_total, matrix_total, "{m}");
        }
    }
}
