use crate::adapter::DuckdbError;
use duckdb::DuckdbConnectionManager;
use duckdb::{params, Transaction};
use error_stack::{IntoReport, Result, ResultExt};
use orca_core::PsqlSettings;
use r2d2::PooledConnection;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{event, instrument, Level};

const POSTGRES_DUCKDB_VERSION_TABLE: &str = "duckdb_data_version";
const HAULS_SCHEMA: &str = "CREATE TABLE
    hauls_matrix_cache (
        catch_location_matrix_index INT NOT NULL,
        catch_location TEXT NOT NULL,
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT NOT NULL,
        fiskeridir_vessel_id INT,
        gear_group_id INT NOT NULL,
        species_group_id INT NOT NULL,
        start_timestamp timestamptz NOT NULL,
        stop_timestamp timestamptz NOT NULL,
        living_weight BIGINT NOT NULL,
    )";

pub struct RefreshRequest(pub Sender<RefreshResponse>);
pub struct RefreshResponse(pub Result<(), DuckdbError>);

pub struct DuckdbRefresher {
    pool: r2d2::Pool<DuckdbConnectionManager>,
    postgres_credentials: String,
    refresh_interval: std::time::Duration,
    refresh_queue: Receiver<RefreshRequest>,
}

pub enum DataSource {
    Hauls,
}

pub enum CreateMode {
    Initial,
    Refresh,
}

impl DataSource {
    fn row_value_name(&self) -> &'static str {
        match self {
            DataSource::Hauls => "hauls",
        }
    }
    fn postgres_version_table_id(&self) -> &'static str {
        match self {
            DataSource::Hauls => "hauls",
        }
    }
}

pub struct RefreshStatus {
    hauls: SourceStatus,
}

pub struct SourceStatus {
    version: u64,
    should_refresh: bool,
}
impl DuckdbRefresher {
    pub fn new(
        pool: r2d2::Pool<DuckdbConnectionManager>,
        postgres_settings: PsqlSettings,
        refresh_interval: std::time::Duration,
        refresh_queue: Receiver<RefreshRequest>,
    ) -> DuckdbRefresher {
        let postgres_credentials = format!(
            "dbname={} user={} host={} password={}",
            postgres_settings
                .db_name
                .clone()
                .unwrap_or("postgres".to_string()),
            postgres_settings.username,
            postgres_settings.ip,
            postgres_settings.password
        );

        DuckdbRefresher {
            pool,
            postgres_credentials,
            refresh_interval,
            refresh_queue,
        }
    }

    pub fn install_postgres_exstension(
        &self,
        conn: &PooledConnection<DuckdbConnectionManager>,
    ) -> Result<(), DuckdbError> {
        // This has to be run prior to starting the transaction
        // as if fails if its excuted during it.
        conn.execute_batch(
            r"
INSTALL postgres;
LOAD postgres;
            ",
        )
        .into_report()
        .change_context(DuckdbError::Query)
        .map(|_| ())
    }

    pub fn create_test_setup(&self) -> Result<(), DuckdbError> {
        let mut conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        self.install_postgres_exstension(&conn)?;

        let tx = conn
            .transaction()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        self.create_hauls(CreateMode::Initial, &tx)?;
        self.add_data_versions(&tx)?;

        tx.commit()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn initial_create(&self) -> Result<(), DuckdbError> {
        let mut conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        self.install_postgres_exstension(&conn)?;

        let tx = conn
            .transaction()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        let table_exists: bool = tx
            .query_row(
                "
SELECT
    COALESCE(
        (
            SELECT
                TRUE
            FROM
                information_schema.tables
            WHERE
                table_name = ?
        ),
        FALSE
    );
                ",
                params![POSTGRES_DUCKDB_VERSION_TABLE],
                |row| row.get(0),
            )
            .into_report()
            .change_context(DuckdbError::Query)?;

        if !table_exists {
            self.create_hauls(CreateMode::Initial, &tx)?;
            self.add_data_versions(&tx)?;
        }

        tx.commit()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        Ok(())
    }

    pub async fn refresh_loop(mut self) {
        let mut interval = tokio::time::interval(self.refresh_interval);

        // Tests rely on the postgres database finishing migration before
        // requesting a refresh and as the first tick resolves instantly
        // we need to skip the first one.
        interval.tick().await;
        loop {
            tokio::select! {
                request = self.refresh_queue.recv() => {
                    match request {
                        Some(v) => {
                            let res = self.refresh_hauls(None);
                            if let Err(e) = v.0.send(RefreshResponse(res)).await {
                                event!(Level::ERROR, "sender half error, exiting refresh_loop: {:?}", e);
                            }

                        }
                        None => {
                            event!(Level::ERROR, "sender half closed, exiting refresh_loop");

                        }
                    }
                }
                _ = interval.tick() => self.do_periodic_refresh(),

            }
        }
    }

    #[instrument(skip_all)]
    fn do_periodic_refresh(&self) {
        match self.refresh_status() {
            Err(e) => {
                event!(
                    Level::ERROR,
                    "failed to check postgres for refresh status: {:?}",
                    e
                );
            }
            Ok(v) => {
                if v.hauls.should_refresh {
                    event!(Level::INFO, "hauls have been modified, starting refresh...",);
                    if let Err(e) = self.refresh_hauls(Some(v.hauls.version)) {
                        event!(Level::ERROR, "failed to set refresh hauls: {:?}", e);
                    }
                }
            }
        }
    }

    #[instrument(skip(self))]
    fn refresh_hauls(&self, new_version: Option<u64>) -> Result<(), DuckdbError> {
        let mut conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )
        .into_report()
        .change_context(DuckdbError::Query)?;

        let tx = conn
            .transaction()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        self.create_hauls(CreateMode::Refresh, &tx)?;

        if let Some(new_version) = new_version {
            self.set_data_source_version(DataSource::Hauls, new_version, &tx)?;
        }

        tx.commit()
            .into_report()
            .change_context(DuckdbError::Connection)?;
        Ok(())
    }

    fn refresh_status(&self) -> Result<RefreshStatus, DuckdbError> {
        let conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )
        .into_report()
        .change_context(DuckdbError::Query)?;

        let postgres_haul_version = self.postgres_data_source_version(&conn, DataSource::Hauls)?;
        let local_haul_version = self.data_source_version(&conn, DataSource::Hauls)?;

        let status = RefreshStatus {
            hauls: SourceStatus {
                version: postgres_haul_version,
                should_refresh: postgres_haul_version > local_haul_version,
            },
        };

        Ok(status)
    }

    fn postgres_data_source_version(
        &self,
        conn: &PooledConnection<DuckdbConnectionManager>,
        source: DataSource,
    ) -> Result<u64, DuckdbError> {
        let version_command = format!(
            "
SELECT
    version
FROM
    POSTGRES_SCAN ('{}', 'public', '{}')
WHERE
    duckdb_data_version_id = ?
            ",
            self.postgres_credentials, POSTGRES_DUCKDB_VERSION_TABLE
        );

        let version: u64 = conn
            .query_row(
                &version_command,
                params![source.postgres_version_table_id()],
                |row| row.get(0),
            )
            .into_report()
            .change_context(DuckdbError::Query)?;
        Ok(version)
    }

    fn data_source_version(
        &self,
        conn: &PooledConnection<DuckdbConnectionManager>,
        source: DataSource,
    ) -> Result<u64, DuckdbError> {
        let version: u64 = conn
            .query_row(
                r#"
SELECT
    "version"
FROM
    data_versions
WHERE
    source = ?
                "#,
                params![source.row_value_name()],
                |row| row.get(0),
            )
            .into_report()
            .change_context(DuckdbError::Query)?;

        Ok(version)
    }

    fn set_data_source_version(
        &self,
        source: DataSource,
        version: u64,
        tx: &Transaction<'_>,
    ) -> Result<(), DuckdbError> {
        tx.execute(
            r#"
UPDATE data_versions
WHERE
    source = ?
SET
    "version" = ?
            "#,
            params![source.row_value_name(), version],
        )
        .into_report()
        .change_context(DuckdbError::Query)?;

        Ok(())
    }

    fn add_data_versions(&self, tx: &Transaction<'_>) -> Result<(), DuckdbError> {
        tx.execute_batch(
            r#"
CREATE TABLE
    data_versions ("version" INT NOT NULL, source VARCHAR PRIMARY KEY,);

INSERT INTO
    data_versions ("version", source)
VALUES
    (0, 'hauls')
ON CONFLICT (source)
DO NOTHING;
            "#,
        )
        .into_report()
        .change_context(DuckdbError::Query)
    }

    fn create_hauls(&self, mode: CreateMode, tx: &Transaction<'_>) -> Result<(), DuckdbError> {
        let postgres_scan_command = format!(
            "
INSERT INTO
    hauls_matrix_cache (
        catch_location_matrix_index,
        catch_location,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        start_timestamp,
        stop_timestamp,
        living_weight
    )
SELECT
    catch_location_matrix_index,
    catch_location,
    matrix_month_bucket,
    vessel_length_group,
    fiskeridir_vessel_id,
    gear_group_id,
    species_group_id,
    start_timestamp,
    stop_timestamp,
    living_weight
FROM
    POSTGRES_SCAN ('{}', 'public', 'hauls_matrix')
            ",
            self.postgres_credentials,
        );

        let queries = match mode {
            CreateMode::Initial => {
                format!("{};{};", HAULS_SCHEMA, postgres_scan_command)
            }
            CreateMode::Refresh => {
                format!(
                    "DROP TABLE hauls_matrix_cache;{};{};",
                    HAULS_SCHEMA, postgres_scan_command
                )
            }
        };
        tx.execute_batch(&queries)
            .into_report()
            .change_context(DuckdbError::Query)
    }
}
