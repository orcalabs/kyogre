use crate::{
    api::matrix_cache::LandingMatrix,
    error::{Error, Result, error::RefreshCommunictionSnafu},
    filter::{HaulFilters, LandingFilters},
    refresher::{DuckdbRefresher, RefreshRequest},
};
use duckdb::DuckdbConnectionManager;
use kyogre_core::{
    HaulMatrixQueryOutput, HaulMatrixXFeature, HaulMatrixYFeature, HaulsMatrix, HaulsMatrixQuery,
    LandingMatrixQuery, LandingMatrixQueryOutput, LandingMatrixXFeature, LandingMatrixYFeature,
    calculate_haul_sum_area_table, calculate_landing_sum_area_table,
};
use orca_core::PsqlSettings;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::mpsc::{self, Sender};
use tracing::{error, info};

#[derive(Clone)]
pub struct DuckdbAdapter {
    pool: r2d2::Pool<DuckdbConnectionManager>,
    cache_mode: CacheMode,
    refresh_queue: Sender<RefreshRequest>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DuckdbSettings {
    pub max_connections: u32,
    pub cache_mode: CacheMode,
    pub storage: CacheStorage,
    #[serde(with = "humantime_serde")]
    pub refresh_interval: std::time::Duration,
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

impl DuckdbAdapter {
    pub fn new(
        settings: &DuckdbSettings,
        postgres_settings: PsqlSettings,
    ) -> Result<(DuckdbAdapter, DuckdbRefresher)> {
        let manager = match &settings.storage {
            CacheStorage::Memory => DuckdbConnectionManager::memory(),
            CacheStorage::Disk(path) => match DuckdbConnectionManager::file(path) {
                Err(e) => {
                    let err: Error = e.into();
                    error!("failed to open duckdb: {err:?}");
                    info!("trying to delete db file and re-open...");
                    std::fs::remove_file(path)?;
                    DuckdbConnectionManager::file(path)
                }
                Ok(v) => Ok(v),
            },
        }?;

        let pool = r2d2::Pool::builder()
            .max_size(settings.max_connections)
            .build(manager)?;

        let (sender, recv) = mpsc::channel(1);

        let refresher = DuckdbRefresher::new(
            pool.clone(),
            postgres_settings,
            settings.refresh_interval,
            recv,
        );

        let adapter = DuckdbAdapter {
            pool,
            cache_mode: settings.cache_mode,
            refresh_queue: sender,
        };

        refresher.initial_create()?;

        Ok((adapter, refresher))
    }

    pub async fn refresh(&self) -> Result<()> {
        let (sender, mut recv) = mpsc::channel(1);
        self.refresh_queue.send(RefreshRequest(sender)).await?;
        match recv.recv().await {
            Some(v) => v.0,
            None => RefreshCommunictionSnafu {}.fail(),
        }
    }

    pub fn hauls_matrix(&self, query: &HaulsMatrixQuery) -> Result<Option<HaulsMatrix>> {
        let res = self.hauls_matrix_impl(query);
        match self.cache_mode {
            CacheMode::MissOnError => match res {
                Ok(v) => Ok(v),
                Err(e) => {
                    error!("failed to get hauls matrix from cache: {e:?}");
                    Ok(None)
                }
            },
            CacheMode::ReturnError => res,
        }
    }

    pub fn landing_matrix(&self, query: &LandingMatrixQuery) -> Result<Option<LandingMatrix>> {
        let res = self.landing_matrix_impl(query);
        match self.cache_mode {
            CacheMode::MissOnError => match res {
                Ok(v) => Ok(v),
                Err(e) => {
                    error!("failed to get landing matrix from cache: {e:?}");
                    Ok(None)
                }
            },
            CacheMode::ReturnError => res,
        }
    }

    fn landing_matrix_impl(&self, params: &LandingMatrixQuery) -> Result<Option<LandingMatrix>> {
        let conn = self.pool.get()?;
        let dates = self.get_landing_matrix(&conn, LandingMatrixXFeature::Date, params)?;
        let length_group =
            self.get_landing_matrix(&conn, LandingMatrixXFeature::VesselLength, params)?;
        let gear_group =
            self.get_landing_matrix(&conn, LandingMatrixXFeature::GearGroup, params)?;
        let species_group =
            self.get_landing_matrix(&conn, LandingMatrixXFeature::SpeciesGroup, params)?;

        match (dates, length_group, gear_group, species_group) {
            (Some(dates), Some(length_group), Some(gear_group), Some(species_group)) => {
                Ok(Some(LandingMatrix {
                    dates,
                    length_group,
                    gear_group,
                    species_group,
                }))
            }
            _ => Ok(None),
        }
    }

    fn hauls_matrix_impl(&self, params: &HaulsMatrixQuery) -> Result<Option<HaulsMatrix>> {
        let conn = self.pool.get()?;
        let dates = self.get_haul_matrix(&conn, HaulMatrixXFeature::Date, params)?;
        let length_group = self.get_haul_matrix(&conn, HaulMatrixXFeature::VesselLength, params)?;
        let gear_group = self.get_haul_matrix(&conn, HaulMatrixXFeature::GearGroup, params)?;
        let species_group =
            self.get_haul_matrix(&conn, HaulMatrixXFeature::SpeciesGroup, params)?;

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

    fn get_landing_matrix(
        &self,
        conn: &r2d2::PooledConnection<DuckdbConnectionManager>,
        x_feature: LandingMatrixXFeature,
        params: &LandingMatrixQuery,
    ) -> Result<Option<Vec<u64>>> {
        let y_feature = if x_feature == params.active_filter {
            LandingMatrixYFeature::CatchLocation
        } else {
            LandingMatrixYFeature::from(params.active_filter)
        };
        let mut sql = format!(
            "
SELECT
    {},
    {},
    SUM(living_weight)
FROM
    landing_matrix_cache
            ",
            x_feature.column_name(),
            y_feature.column_name()
        );

        let filter = LandingFilters::new(params, x_feature, y_feature);
        sql.push_str(&filter.query_string());
        sql.push_str("group by 1,2");

        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query([])?;

        let data = get_landing_matrix_output(rows)?;

        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(calculate_landing_sum_area_table(
                x_feature, y_feature, data,
            )?))
        }
    }

    fn get_haul_matrix(
        &self,
        conn: &r2d2::PooledConnection<DuckdbConnectionManager>,
        x_feature: HaulMatrixXFeature,
        params: &HaulsMatrixQuery,
    ) -> Result<Option<Vec<u64>>> {
        let y_feature = if x_feature == params.active_filter {
            HaulMatrixYFeature::CatchLocation
        } else {
            HaulMatrixYFeature::from(params.active_filter)
        };
        let mut sql = format!(
            "
SELECT
    {},
    {},
    SUM(living_weight)
FROM
    hauls_matrix_cache
            ",
            x_feature.column_name(),
            y_feature.column_name()
        );

        let filter = HaulFilters::new(params, x_feature, y_feature);
        sql.push_str(&filter.query_string());
        sql.push_str("group by 1,2");

        let mut stmt = conn.prepare(&sql)?;

        let rows = stmt.query([])?;

        let data = get_haul_matrix_output(rows)?;

        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(calculate_haul_sum_area_table(
                x_feature, y_feature, data,
            )?))
        }
    }
}

fn get_landing_matrix_output(mut rows: duckdb::Rows<'_>) -> Result<Vec<LandingMatrixQueryOutput>> {
    let mut data = Vec::new();
    while let Some(row) = rows.next()? {
        data.push(LandingMatrixQueryOutput {
            x_index: row.get(0)?,
            y_index: row.get(1)?,
            sum_living: row.get(2)?,
        });
    }

    Ok(data)
}

fn get_haul_matrix_output(mut rows: duckdb::Rows<'_>) -> Result<Vec<HaulMatrixQueryOutput>> {
    let mut data = Vec::new();
    while let Some(row) = rows.next()? {
        data.push(HaulMatrixQueryOutput {
            x_index: row.get(0)?,
            y_index: row.get(1)?,
            sum_living: row.get(2)?,
        });
    }

    Ok(data)
}
