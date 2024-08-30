use crate::adapter::DuckdbError;
use duckdb::DuckdbConnectionManager;
use duckdb::{params, Transaction};
use error_stack::{Result, ResultExt};
use orca_core::PsqlSettings;
use r2d2::PooledConnection;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info, instrument};

const POSTGRES_DUCKDB_VERSION_TABLE: &str = "duckdb_data_version";
const HAULS_SCHEMA: &str = "CREATE TABLE
    hauls_matrix_cache (
        catch_location_matrix_index INT NOT NULL,
        catch_location_id TEXT NOT NULL,
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT NOT NULL,
        fiskeridir_vessel_id INT,
        gear_group_id INT NOT NULL,
        species_group_id INT NOT NULL,
        living_weight DOUBLE NOT NULL,
        species_group_weight_percentage_of_haul DOUBLE,
        is_majority_species_group_of_haul BOOLEAN
    )";
const LANDING_SCHEMA: &str = "CREATE TABLE
    landing_matrix_cache (
        landing_id VARCHAR NOT NULL,
        catch_location_matrix_index INT NOT NULL,
        catch_location_id VARCHAR NOT NULL ,
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT,
        fiskeridir_vessel_id INT,
        gear_group_id INT NOT NULL,
        species_group_id INT NOT NULL,
        living_weight DOUBLE NOT NULL,
        PRIMARY KEY (landing_id, species_group_id)
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
    Landings,
}

pub enum CreateMode {
    Initial,
    Refresh,
}

impl DataSource {
    fn row_value_name(&self) -> &'static str {
        match self {
            DataSource::Hauls => "hauls",
            DataSource::Landings => "landings",
        }
    }
    fn postgres_version_table_id(&self) -> &'static str {
        match self {
            DataSource::Hauls => "hauls",
            DataSource::Landings => "landings",
        }
    }
}

pub struct RefreshStatus {
    hauls: SourceStatus,
    landings: SourceStatus,
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
            "postgresql://{}{}@{}/{}{}",
            postgres_settings.username,
            postgres_settings
                .password
                .map(|p| format!(":{p}"))
                .unwrap_or_default(),
            postgres_settings.ip,
            postgres_settings
                .db_name
                .clone()
                .unwrap_or("postgres".to_string()),
            postgres_settings
                .application_name
                .map(|n| format!("?application_name={n}"))
                .unwrap_or_default(),
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
        .change_context(DuckdbError::Query)
        .map(|_| ())
    }

    #[instrument(skip_all)]
    pub fn initial_create(&self) -> Result<(), DuckdbError> {
        let mut conn = self.pool.get().change_context(DuckdbError::Connection)?;

        self.install_postgres_exstension(&conn)?;

        let tx = conn.transaction().change_context(DuckdbError::Connection)?;

        tx.execute(
            &format!(
                "ATTACH '{}' AS postgres_db (TYPE postgres);",
                self.postgres_credentials
            ),
            [],
        )
        .change_context(DuckdbError::Query)?;

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
                table_name = 'data_versions'
        ),
        FALSE
    );
                ",
                params![],
                |row| row.get(0),
            )
            .change_context(DuckdbError::Query)?;

        if !table_exists {
            self.create_hauls(CreateMode::Initial, &tx)?;
            self.create_landings(CreateMode::Initial, &tx)?;
            self.add_data_versions(&tx)?;
        }

        tx.commit().change_context(DuckdbError::Connection)?;

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
                        Some(v) => self.do_periodic_refresh(Some(v.0)).await,
                        None => {
                            error!("sender half closed, exiting refresh_loop");

                        }
                    }
                }
                _ = interval.tick() => self.do_periodic_refresh(None).await,
            }
        }
    }

    #[instrument(skip_all)]
    async fn do_periodic_refresh(&self, response_channel: Option<Sender<RefreshResponse>>) {
        let res = match self.refresh_status() {
            Err(e) => {
                error!("failed to check postgres for refresh status: {e:?}");
                Err(e)
            }
            Ok(v) => {
                let res = if v.hauls.should_refresh {
                    info!("hauls have been modified, starting refresh...");
                    match self.refresh_hauls(Some(v.hauls.version)) {
                        Err(e) => {
                            error!("failed to set refresh hauls: {e:?}");
                            Err(e)
                        }
                        Ok(v) => Ok(v),
                    }
                } else {
                    Ok(())
                };
                let res2 = if v.landings.should_refresh {
                    info!("landings have been modified, starting refresh...");
                    match self.refresh_landings(Some(v.landings.version)) {
                        Err(e) => {
                            error!("failed to set refresh landings: {e:?}");
                            Err(e)
                        }
                        Ok(v) => Ok(v),
                    }
                } else {
                    Ok(())
                };

                match (res, res2) {
                    (_, Err(e)) => Err(e),
                    (Err(e), _) => Err(e),
                    (_, _) => Ok(()),
                }
            }
        };

        if let Some(sender) = response_channel {
            if let Err(e) = sender.send(RefreshResponse(res)).await {
                error!("sender half error, exiting refresh_loop: {e:?}");
            }
        }
    }

    #[instrument(skip(self))]
    fn refresh_landings(&self, new_version: Option<u64>) -> Result<(), DuckdbError> {
        let mut conn = self.pool.get().change_context(DuckdbError::Connection)?;

        conn.execute("DELETE FROM landing_matrix_cache", [])
            .change_context(DuckdbError::Query)?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )
        .change_context(DuckdbError::Query)?;

        let tx = conn.transaction().change_context(DuckdbError::Connection)?;

        self.create_landings(CreateMode::Refresh, &tx)?;

        if let Some(new_version) = new_version {
            self.set_data_source_version(DataSource::Landings, new_version, &tx)?;
        }

        tx.commit().change_context(DuckdbError::Connection)?;
        Ok(())
    }

    #[instrument(skip(self))]
    fn refresh_hauls(&self, new_version: Option<u64>) -> Result<(), DuckdbError> {
        let mut conn = self.pool.get().change_context(DuckdbError::Connection)?;

        conn.execute("DELETE FROM hauls_matrix_cache", [])
            .change_context(DuckdbError::Query)?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )
        .change_context(DuckdbError::Query)?;

        let tx = conn.transaction().change_context(DuckdbError::Connection)?;

        self.create_hauls(CreateMode::Refresh, &tx)?;

        if let Some(new_version) = new_version {
            self.set_data_source_version(DataSource::Hauls, new_version, &tx)?;
        }

        tx.commit().change_context(DuckdbError::Connection)?;
        Ok(())
    }

    fn refresh_status(&self) -> Result<RefreshStatus, DuckdbError> {
        let conn = self.pool.get().change_context(DuckdbError::Connection)?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )
        .change_context(DuckdbError::Query)?;

        let postgres_haul_version = self.postgres_data_source_version(&conn, DataSource::Hauls)?;
        let local_haul_version = self.data_source_version(&conn, DataSource::Hauls)?;

        let postgres_landing_version =
            self.postgres_data_source_version(&conn, DataSource::Landings)?;
        let local_landing_version = self.data_source_version(&conn, DataSource::Landings)?;

        let status = RefreshStatus {
            hauls: SourceStatus {
                version: postgres_haul_version,
                should_refresh: postgres_haul_version > local_haul_version,
            },
            landings: SourceStatus {
                version: postgres_landing_version,
                should_refresh: postgres_landing_version > local_landing_version,
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
    postgres_db.{}
WHERE
    duckdb_data_version_id = ?
            ",
            POSTGRES_DUCKDB_VERSION_TABLE
        );

        let version: u64 = conn
            .query_row(
                &version_command,
                params![source.postgres_version_table_id()],
                |row| row.get(0),
            )
            .change_context(DuckdbError::Query)?;
        Ok(version)
    }

    fn postgres_data_source_version_tx(
        &self,
        tx: &Transaction<'_>,
        source: DataSource,
    ) -> Result<u64, DuckdbError> {
        let version_command = format!(
            "
SELECT
    version
FROM
    postgres_db.{}
WHERE
    duckdb_data_version_id = ?
            ",
            POSTGRES_DUCKDB_VERSION_TABLE
        );

        let version: u64 = tx
            .query_row(
                &version_command,
                params![source.postgres_version_table_id()],
                |row| row.get(0),
            )
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
SET
    "version" = ?
WHERE
    source = ?
            "#,
            params![version, source.row_value_name()],
        )
        .change_context(DuckdbError::Query)?;

        Ok(())
    }

    fn add_data_versions(&self, tx: &Transaction<'_>) -> Result<(), DuckdbError> {
        let postgres_haul_version = self.postgres_data_source_version_tx(tx, DataSource::Hauls)?;
        let postgres_landing_version =
            self.postgres_data_source_version_tx(tx, DataSource::Landings)?;

        tx.execute_batch(
            r#"
CREATE TABLE
    data_versions ("version" INT NOT NULL, source VARCHAR PRIMARY KEY,);
            "#,
        )
        .change_context(DuckdbError::Query)?;

        tx.execute(
            r#"
INSERT INTO
    data_versions ("version", source)
VALUES
    (?, 'landings')
ON CONFLICT (source) DO NOTHING;
            "#,
            [postgres_landing_version],
        )
        .change_context(DuckdbError::Query)?;

        tx.execute(
            r#"
INSERT INTO
    data_versions ("version", source)
VALUES
    (?, 'hauls')
ON CONFLICT (source) DO NOTHING;
            "#,
            [postgres_haul_version],
        )
        .change_context(DuckdbError::Query)?;

        Ok(())
    }

    #[instrument(skip_all)]
    fn create_landings(&self, mode: CreateMode, tx: &Transaction<'_>) -> Result<(), DuckdbError> {
        let postgres_scan_command = "
INSERT INTO
    landing_matrix_cache (
        landing_id,
        catch_location_matrix_index,
        catch_location_id,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        living_weight
    )
SELECT
   landing_id,
   catch_location_matrix_index,
   catch_location_id,
   matrix_month_bucket,
   vessel_length_group,
   fiskeridir_vessel_id,
   gear_group_id,
   species_group_id,
   living_weight
FROM
    postgres_db.landing_matrix"
            .to_string();

        let queries = match mode {
            CreateMode::Initial => {
                format!(
                    "DROP TABLE IF EXISTS landing_matrix_cache;{};{};",
                    LANDING_SCHEMA, postgres_scan_command
                )
            }
            CreateMode::Refresh => postgres_scan_command,
        };
        tx.execute_batch(&queries)
            .change_context(DuckdbError::Query)
    }

    #[instrument(skip_all)]
    fn create_hauls(&self, mode: CreateMode, tx: &Transaction<'_>) -> Result<(), DuckdbError> {
        let postgres_scan_command = "
INSERT INTO
    hauls_matrix_cache (
        catch_location_matrix_index,
        catch_location_id,
        matrix_month_bucket,
        vessel_length_group,
        fiskeridir_vessel_id,
        gear_group_id,
        species_group_id,
        living_weight,
        species_group_weight_percentage_of_haul,
        is_majority_species_group_of_haul
    )
SELECT
    catch_location_matrix_index,
    catch_location,
    matrix_month_bucket,
    vessel_length_group,
    fiskeridir_vessel_id,
    gear_group_id,
    species_group_id,
    living_weight,
    species_group_weight_percentage_of_haul,
    is_majority_species_group_of_haul
FROM
    postgres_db.hauls_matrix"
            .to_string();

        let queries = match mode {
            CreateMode::Initial => {
                format!(
                    "DROP TABLE IF EXISTS hauls_matrix_cache;{};{};",
                    HAULS_SCHEMA, postgres_scan_command
                )
            }
            CreateMode::Refresh => postgres_scan_command,
        };
        tx.execute_batch(&queries)
            .change_context(DuckdbError::Query)
    }
}
