use std::path::PathBuf;

use duckdb::DuckdbConnectionManager;
use error_stack::{Context, IntoReport, Result, ResultExt};
use kyogre_core::*;
use orca_core::PsqlSettings;
use serde::Deserialize;
use tracing::{event, instrument, Level};

use crate::Cache;

#[derive(Clone)]
pub struct DuckdbAdapter {
    pool: r2d2::Pool<DuckdbConnectionManager>,
    postgres_settings: PsqlSettings,
    cache_mode: CacheMode,
}

#[derive(Clone, Debug, Copy, Deserialize)]
pub enum CacheMode {
    MissOnError,
    // We will return an api error if the cache returns an error,
    // used for testing purposes
    ReturnError,
}

#[derive(Clone, Debug, Deserialize)]
pub enum CacheStorage {
    Memory,
    Disk(PathBuf),
}

#[derive(Clone, Debug, Deserialize)]
pub struct DuckdbSettings {
    pub max_connections: u32,
    pub mode: CacheMode,
    pub storage: CacheStorage,
}

impl DuckdbAdapter {
    pub fn new(
        settings: &DuckdbSettings,
        postgres_settings: PsqlSettings,
    ) -> Result<DuckdbAdapter, DuckdbError> {
        let manager = match &settings.storage {
            CacheStorage::Memory => DuckdbConnectionManager::memory()
                .into_report()
                .change_context(DuckdbError::Connection),
            CacheStorage::Disk(path) => DuckdbConnectionManager::file(path)
                .into_report()
                .change_context(DuckdbError::Connection),
        }?;
        let pool = r2d2::Pool::builder()
            .max_size(settings.max_connections)
            .build(manager)
            .into_report()
            .change_context(DuckdbError::Connection)?;

        Ok(DuckdbAdapter {
            pool,
            postgres_settings,
            cache_mode: settings.mode,
        })
    }

    #[instrument(skip_all)]
    pub fn refresh_hauls_cache_impl(&self) -> Result<(), DuckdbError> {
        let conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;

        conn.execute_batch(
            r"
INSTALL postgres;
LOAD postgres;
DROP TABLE IF EXISTS hauls_matrix_cache;
CREATE TABLE
    hauls_matrix_cache (
        catch_location_start_matrix_index INT NOT NULL,
        catch_location_start TEXT NOT NULL,
        matrix_month_bucket INT NOT NULL,
        vessel_length_group INT NOT NULL,
        fiskeridir_vessel_id INT,
        gear_group_id INT NOT NULL,
        species_group_id INT NOT NULL,
        start_timestamp timestamptz NOT NULL,
        stop_timestamp timestamptz NOT NULL,
        living_weight DOUBLE NOT NULL,
    );
            ",
        )
        .into_report()
        .change_context(DuckdbError::Query)?;

        let postgres_scan_command = format!(
            "
select catch_location_start_matrix_index, catch_location_start, matrix_month_bucket, vessel_length_group,
 fiskeridir_vessel_id ,gear_group_id, species_group_id, start_timestamp, stop_timestamp,
 living_weight from POSTGRES_SCAN('dbname={} user={} host={} password={}', 'public', 'hauls_matrix')",
            self.postgres_settings
                .db_name
                .clone()
                .unwrap_or("postgres".to_string()),
            self.postgres_settings.username,
            self.postgres_settings.ip,
            self.postgres_settings.password,
        );

        conn.execute(
            &format!("insert into hauls_matrix_cache(catch_location_start_matrix_index, catch_location_start,
                matrix_month_bucket, vessel_length_group, fiskeridir_vessel_id, gear_group_id, species_group_id,
                start_timestamp, stop_timestamp, living_weight) {}", postgres_scan_command),
            [],
        )
        .into_report()
        .change_context(DuckdbError::Query)?;

        Ok(())
    }
    fn get_matrixes(&self, params: &HaulsMatrixQuery) -> Result<Option<HaulsMatrix>, DuckdbError> {
        let conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;
        let dates = self.get_matrix(
            &conn,
            if matches!(params.active_filter, ActiveHaulsFilter::Date) {
                HaulMatrixFeatures::CatchLocation
            } else {
                HaulMatrixFeatures::Date
            },
            params,
        )?;
        let length_group = self.get_matrix(
            &conn,
            if matches!(params.active_filter, ActiveHaulsFilter::VesselLength) {
                HaulMatrixFeatures::CatchLocation
            } else {
                HaulMatrixFeatures::VesselLength
            },
            params,
        )?;
        let gear_group = self.get_matrix(
            &conn,
            if matches!(params.active_filter, ActiveHaulsFilter::GearGroup) {
                HaulMatrixFeatures::CatchLocation
            } else {
                HaulMatrixFeatures::GearGroup
            },
            params,
        )?;
        let species_group = self.get_matrix(
            &conn,
            if matches!(params.active_filter, ActiveHaulsFilter::SpeciesGroup) {
                HaulMatrixFeatures::CatchLocation
            } else {
                HaulMatrixFeatures::SpeciesGroup
            },
            params,
        )?;

        match (dates, length_group, gear_group, species_group) {
            (Some(dates), Some(length_group), Some(gear_group), Some(species_group)) => {
                Ok(Some(HaulsMatrix {
                    dates,
                    length_group,
                    gear_group,
                    species_group,
                }))
            }
            _ => Ok(None),
        }
    }

    fn get_matrix(
        &self,
        conn: &r2d2::PooledConnection<DuckdbConnectionManager>,
        x_feature: HaulMatrixFeatures,
        params: &HaulsMatrixQuery,
    ) -> Result<Option<Vec<u64>>, DuckdbError> {
        let y_feature = HaulMatrixFeatures::from(params.active_filter);
        let mut sql = format!(
            "select {}, {}, sum(living_weight) from hauls_matrix_cache ",
            x_feature.column_name(),
            y_feature.column_name()
        );

        push_where_statements(&mut sql, params);

        sql.push_str("group by 1,2");

        let mut stmt = conn
            .prepare(&sql)
            .into_report()
            .change_context(DuckdbError::Query)?;

        let rows = stmt
            .query([])
            .into_report()
            .change_context(DuckdbError::Query)?;

        let data = get_matrix_output(rows)
            .into_report()
            .change_context(DuckdbError::Conversion)?;

        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                calculate_sum_area_table(x_feature, y_feature, data)
                    .change_context(DuckdbError::Conversion)?,
            ))
        }
    }
}

fn get_matrix_output(
    mut rows: duckdb::Rows<'_>,
) -> std::result::Result<Vec<MatrixQueryOutput>, duckdb::Error> {
    let mut data = Vec::new();
    while let Some(row) = rows.next()? {
        data.push(MatrixQueryOutput {
            x_index: row.get(0)?,
            y_index: row.get(1)?,
            sum_living: row.get(2)?,
        });
    }

    Ok(data)
}

impl CacheOutboundPort for DuckdbAdapter {
    fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> Result<Option<HaulsMatrix>, QueryError> {
        let res = self.get_matrixes(query).change_context(QueryError);
        match self.cache_mode {
            CacheMode::MissOnError => match res {
                Ok(v) => Ok(v),
                Err(e) => {
                    event!(
                        Level::ERROR,
                        "failed to get hauls matrix from cache: {:?}",
                        e
                    );
                    Ok(None)
                }
            },
            CacheMode::ReturnError => res,
        }
    }
}

impl Cache for DuckdbAdapter {}

fn push_where_statements(query: &mut String, params: &HaulsMatrixQuery) {
    let mut first = true;

    if let Some(months) = &params.months {
        if first {
            first = false;
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        query.push_str(&format!("matrix_month_bucket = ANY ({:?}) ", months));
    }
    if let Some(catch_locations) = &params.catch_locations {
        if first {
            first = false;
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        let mut filter = String::new();
        for c in catch_locations {
            filter.push_str(&format!("'{}',", c.as_ref()));
        }
        filter.pop();
        query.push_str(&format!("catch_location_start = ANY ([{filter}]) ",));
    }
    if let Some(gear_group_ids) = &params.gear_group_ids {
        if first {
            first = false;
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        query.push_str(&format!(
            "gear_group_id = ANY ({:?}) ",
            gear_group_ids
                .iter()
                .map(|v| *v as i32)
                .collect::<Vec<i32>>()
        ));
    }
    if let Some(species_group_ids) = &params.species_group_ids {
        if first {
            first = false;
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        query.push_str(&format!(
            "species_group_id = ANY ({:?}) ",
            species_group_ids
        ));
    }
    if let Some(vessel_length_groups) = &params.vessel_length_groups {
        if first {
            first = false;
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        query.push_str(&format!(
            "vessel_length_group = ANY ({:?}) ",
            vessel_length_groups
                .iter()
                .map(|v| *v as i32)
                .collect::<Vec<i32>>()
        ));
    }
    if let Some(vessel_ids) = &params.vessel_ids {
        if first {
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        query.push_str(&format!(
            "fiskeridir_vessel_id = ANY ({:?}) ",
            vessel_ids.iter().map(|v| v.0).collect::<Vec<i64>>()
        ));
    }
}

#[derive(Debug)]
pub enum DuckdbError {
    Connection,
    Query,
    Conversion,
}

impl std::fmt::Display for DuckdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuckdbError::Connection => f.write_str("failed to establish connection with duckdb"),
            DuckdbError::Query => f.write_str("failed to perfom a query"),
            DuckdbError::Conversion => f.write_str("failed to convert output of query"),
        }
    }
}

impl Context for DuckdbError {}
