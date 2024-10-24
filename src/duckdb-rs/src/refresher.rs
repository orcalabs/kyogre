use crate::error::Result;
use duckdb::DuckdbConnectionManager;
use duckdb::{params, Transaction};
use kyogre_core::IsTimeout;
use orca_core::PsqlSettings;
use r2d2::PooledConnection;
use std::u64;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info, instrument};

const POSTGRES_DUCKDB_VERSION_TABLE: &str = "duckdb_data_version";
const HAULS_SCHEMA: &str = "CREATE TABLE
    hauls_matrix_cache (
        haul_id INT NOT NULL,
        catch_location_matrix_index INT NOT NULL,
        catch_location TEXT NOT NULL,
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT NOT NULL,
        fiskeridir_vessel_id INT,
        gear_group_id INT NOT NULL,
        species_group_id INT NOT NULL,
        living_weight DOUBLE NOT NULL,
        species_group_weight_percentage_of_haul DOUBLE,
        is_majority_species_group_of_haul BOOLEAN,
        PRIMARY KEY(haul_id, species_group_id, catch_location)
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
pub struct RefreshResponse(pub Result<()>);

pub struct DuckdbRefresher {
    pool: r2d2::Pool<DuckdbConnectionManager>,
    postgres_credentials: String,
    refresh_interval: std::time::Duration,
    refresh_queue: Receiver<RefreshRequest>,
    landing_version: u64,
    haul_version: u64,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DataSource {
    Hauls,
    Landings,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Postgres {
    Hauls,
    Landings,
}

#[derive(Debug)]
pub enum CreateMode {
    Initial,
    Refresh { matrix_month_bucket: u64 },
}

impl DataSource {
    fn postgres_version_table_id(&self) -> &'static str {
        match self {
            DataSource::Hauls => "Hauls",
            DataSource::Landings => "Landings",
        }
    }
}

#[derive(Debug)]
pub struct RefreshStatus {
    hauls: Option<SourceStatus>,
    landings: Option<SourceStatus>,
}

#[derive(Debug)]
pub struct SourceStatus {
    postgres_version: u64,
    matrix_month_bucket: u64,
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
            landing_version: 0,
            haul_version: 0,
        }
    }

    pub fn install_postgres_exstension(
        &self,
        conn: &PooledConnection<DuckdbConnectionManager>,
    ) -> Result<()> {
        // This has to be run prior to starting the transaction
        // as if fails if its excuted during it.
        Ok(conn
            .execute_batch(
                r"
INSTALL postgres;
LOAD postgres;
            ",
            )
            .map(|_| ())?)
    }

    #[instrument(skip_all)]
    pub fn initial_create(&mut self) -> Result<()> {
        let mut conn = self.pool.get()?;

        self.install_postgres_exstension(&conn)?;

        let tx = conn.transaction()?;

        tx.execute(
            &format!(
                "ATTACH '{}' AS postgres_db (TYPE postgres);",
                self.postgres_credentials
            ),
            [],
        )?;

        self.create_hauls(CreateMode::Initial, &tx)?;
        self.create_landings(CreateMode::Initial, &tx)?;
        let haul_version = self.postgres_data_source_version_tx(&tx, DataSource::Hauls)?;
        let landing_version = self.postgres_data_source_version_tx(&tx, DataSource::Landings)?;

        tx.commit()?;

        self.landing_version = landing_version;
        self.haul_version = haul_version;

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
    async fn do_periodic_refresh(&mut self, response_channel: Option<Sender<RefreshResponse>>) {
        let mut attempt = 0;
        loop {
            match self.do_periodic_refresh_impl() {
                Ok(_) => {
                    if let Some(ref sender) = response_channel {
                        if let Err(e) = sender.send(RefreshResponse(Ok(()))).await {
                            error!("sender half error, exiting refresh_loop: {e:?}");
                        }
                    }
                    break;
                }
                Err(e) if e.is_timeout() => {
                    attempt = attempt + 1;
                    if attempt > 3 {
                        error!("failed periodic refresh: {e:?}");
                        break;
                    }
                }
                Err(e) => {
                    error!("failed periodic refresh: {e:?}");
                    break;
                }
            }
        }
    }

    fn do_periodic_refresh_impl(&mut self) -> Result<()> {
        let status = self.refresh_status()?;

        let res = match status.hauls {
            Some(hauls) if hauls.postgres_version > self.haul_version => {
                info!("hauls have been modified, starting refresh...");
                self.refresh_hauls(hauls)
            }
            _ => Ok(()),
        };

        let res2 = match status.landings {
            Some(landings) if landings.postgres_version > self.landing_version => {
                info!("landings have been modified, starting refresh...");
                self.refresh_landings(landings)
            }
            _ => Ok(()),
        };

        res.and(res2)
    }

    #[instrument(skip(self))]
    fn refresh_landings(&mut self, status: SourceStatus) -> Result<()> {
        let mut conn = self.pool.get()?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )?;

        let tx = conn.transaction()?;

        self.create_landings(
            CreateMode::Refresh {
                matrix_month_bucket: status.matrix_month_bucket,
            },
            &tx,
        )?;

        self.landing_version = status.matrix_month_bucket;

        tx.commit()?;
        Ok(())
    }

    #[instrument(skip(self))]
    fn refresh_hauls(&mut self, status: SourceStatus) -> Result<()> {
        let mut conn = self.pool.get()?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )?;

        let tx = conn.transaction()?;

        self.create_hauls(
            CreateMode::Refresh {
                matrix_month_bucket: status.matrix_month_bucket,
            },
            &tx,
        )?;

        self.haul_version = status.postgres_version;

        tx.commit()?;
        Ok(())
    }

    fn refresh_status(&self) -> Result<RefreshStatus> {
        let conn = self.pool.get()?;

        conn.execute(
            r"
LOAD postgres;
            ",
            [],
        )?;

        let hauls_status = self.postgres_data_source_version(&conn, DataSource::Hauls)?;
        let landings_status = self.postgres_data_source_version(&conn, DataSource::Landings)?;

        let status = RefreshStatus {
            hauls: hauls_status,
            landings: landings_status,
        };

        Ok(status)
    }

    fn postgres_data_source_version(
        &self,
        conn: &PooledConnection<DuckdbConnectionManager>,
        source: DataSource,
    ) -> Result<Option<SourceStatus>> {
        let version_command = format!(
            "
SELECT
    version,
    matrix_month_bucket
FROM
    postgres_db.{}
WHERE
    duckdb_data_version_id = ?
AND
    version > ?
ORDER BY matrix_month_bucket
LIMIT 1
            ",
            POSTGRES_DUCKDB_VERSION_TABLE
        );

        match conn.query_row(
            &version_command,
            params![source.postgres_version_table_id(), self.landing_version],
            |row| Ok((row.get(0), row.get(1))),
        ) {
            Ok((postgres_version, matrix_month_bucket)) => Ok(Some(SourceStatus {
                postgres_version: postgres_version?,
                matrix_month_bucket: matrix_month_bucket?,
            })),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn postgres_data_source_version_tx(
        &self,
        tx: &Transaction<'_>,
        source: DataSource,
    ) -> Result<u64> {
        let version_command = format!(
            "
SELECT
    version
FROM
    postgres_db.{}
WHERE
    duckdb_data_version_id = ?
ORDER BY version DESC
LIMIT 1
            ",
            POSTGRES_DUCKDB_VERSION_TABLE
        );

        match tx.query_row(
            &version_command,
            params![source.postgres_version_table_id()],
            |row| row.get(0),
        ) {
            Ok(version) => Ok(version),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    #[instrument(skip_all)]
    fn create_landings(&self, mode: CreateMode, tx: &Transaction<'_>) -> Result<()> {
        let mut postgres_scan_command = "
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

        match mode {
            CreateMode::Initial => {
                Ok(tx.execute_batch(&format!("{};{};", LANDING_SCHEMA, postgres_scan_command))?)
            }
            CreateMode::Refresh {
                matrix_month_bucket,
            } => {
                tx.execute(
                    "DELETE FROM landing_matrix_cache WHERE matrix_month_bucket >= ?",
                    params![matrix_month_bucket],
                )?;
                postgres_scan_command.push_str(&format!(" WHERE matrix_month_bucket >= ?"));
                tx.execute(&postgres_scan_command, params![matrix_month_bucket])?;
                Ok(())
            }
        }
    }

    #[instrument(skip_all)]
    fn create_hauls(&self, mode: CreateMode, tx: &Transaction<'_>) -> Result<()> {
        let mut postgres_scan_command = "
INSERT INTO
    hauls_matrix_cache (
        haul_id,
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
    )
SELECT
    haul_id,
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

        match mode {
            CreateMode::Initial => {
                Ok(tx.execute_batch(&format!("{};{};", HAULS_SCHEMA, postgres_scan_command))?)
            }
            CreateMode::Refresh {
                matrix_month_bucket,
            } => {
                tx.execute(
                    "DELETE FROM hauls_matrix_cache WHERE matrix_month_bucket >= ?",
                    params![matrix_month_bucket],
                )?;
                postgres_scan_command.push_str(&format!(" WHERE matrix_month_bucket >= ?"));
                tx.execute(&postgres_scan_command, params![matrix_month_bucket])?;
                Ok(())
            }
        }
    }
}
