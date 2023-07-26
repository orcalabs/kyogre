use duckdb::DuckdbConnectionManager;
use error_stack::{report, Context, IntoReport, Result, ResultExt};
use kyogre_core::*;
use orca_core::PsqlSettings;
use serde::Deserialize;
use std::path::PathBuf;
use tokio::sync::mpsc::{self, Sender};
use tracing::{event, Level};

use crate::refresher::{DuckdbRefresher, RefreshRequest};

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
    ) -> Result<DuckdbAdapter, DuckdbError> {
        let manager = match &settings.storage {
            CacheStorage::Memory => DuckdbConnectionManager::memory()
                .into_report()
                .change_context(DuckdbError::Connection),
            CacheStorage::Disk(path) => match DuckdbConnectionManager::file(path) {
                Err(e) => {
                    event!(Level::ERROR, "failed to open duckdb: {}", e);
                    event!(Level::INFO, "trying to delete db file and re-open...");
                    std::fs::remove_file(path)
                        .into_report()
                        .change_context(DuckdbError::Connection)?;
                    DuckdbConnectionManager::file(path)
                        .into_report()
                        .change_context(DuckdbError::Connection)
                }
                Ok(v) => Ok(v),
            },
        }?;

        let pool = r2d2::Pool::builder()
            .max_size(settings.max_connections)
            .build(manager)
            .into_report()
            .change_context(DuckdbError::Connection)?;

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
        tokio::spawn(refresher.refresh_loop());

        Ok(adapter)
    }

    pub async fn refresh(&self) -> Result<(), DuckdbError> {
        let (sender, mut recv) = mpsc::channel(1);
        match self.refresh_queue.send(RefreshRequest(sender)).await {
            Err(e) => Err(report!(DuckdbError::RefreshCommunication).attach_printable(e)),
            Ok(_) => match recv.recv().await {
                Some(v) => v.0,
                None => Err(report!(DuckdbError::RefreshCommunication)),
            },
        }
    }

    pub fn hauls_matrix(
        &self,
        query: &HaulsMatrixQuery,
    ) -> Result<Option<HaulsMatrix>, QueryError> {
        let res = self.hauls_matrix_impl(query).change_context(QueryError);
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

    pub fn landing_matrix(
        &self,
        query: &LandingMatrixQuery,
    ) -> Result<Option<LandingMatrix>, QueryError> {
        let res = self.landing_matrix_impl(query).change_context(QueryError);
        match self.cache_mode {
            CacheMode::MissOnError => match res {
                Ok(v) => Ok(v),
                Err(e) => {
                    event!(
                        Level::ERROR,
                        "failed to get landing matrix from cache: {:?}",
                        e
                    );
                    Ok(None)
                }
            },
            CacheMode::ReturnError => res,
        }
    }

    fn landing_matrix_impl(
        &self,
        params: &LandingMatrixQuery,
    ) -> Result<Option<LandingMatrix>, DuckdbError> {
        let conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;
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

    fn hauls_matrix_impl(
        &self,
        params: &HaulsMatrixQuery,
    ) -> Result<Option<HaulsMatrix>, DuckdbError> {
        let conn = self
            .pool
            .get()
            .into_report()
            .change_context(DuckdbError::Connection)?;
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
    ) -> Result<Option<Vec<u64>>, DuckdbError> {
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

        push_landing_where_statements(&mut sql, params, x_feature);

        sql.push_str("group by 1,2");

        let mut stmt = conn
            .prepare(&sql)
            .into_report()
            .change_context(DuckdbError::Query)?;

        let rows = stmt
            .query([])
            .into_report()
            .change_context(DuckdbError::Query)?;

        let data = get_landing_matrix_output(rows)
            .into_report()
            .change_context(DuckdbError::Conversion)?;

        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                calculate_landing_sum_area_table(x_feature, y_feature, data)
                    .change_context(DuckdbError::Conversion)?,
            ))
        }
    }

    fn get_haul_matrix(
        &self,
        conn: &r2d2::PooledConnection<DuckdbConnectionManager>,
        x_feature: HaulMatrixXFeature,
        params: &HaulsMatrixQuery,
    ) -> Result<Option<Vec<u64>>, DuckdbError> {
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

        push_haul_where_statements(&mut sql, params, x_feature);

        sql.push_str("group by 1,2");

        let mut stmt = conn
            .prepare(&sql)
            .into_report()
            .change_context(DuckdbError::Query)?;

        let rows = stmt
            .query([])
            .into_report()
            .change_context(DuckdbError::Query)?;

        let data = get_haul_matrix_output(rows)
            .into_report()
            .change_context(DuckdbError::Conversion)?;

        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                calculate_haul_sum_area_table(x_feature, y_feature, data)
                    .change_context(DuckdbError::Conversion)?,
            ))
        }
    }
}

fn get_landing_matrix_output(
    mut rows: duckdb::Rows<'_>,
) -> std::result::Result<Vec<LandingMatrixQueryOutput>, duckdb::Error> {
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

fn get_haul_matrix_output(
    mut rows: duckdb::Rows<'_>,
) -> std::result::Result<Vec<HaulMatrixQueryOutput>, duckdb::Error> {
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

fn push_landing_where_statements(
    query: &mut String,
    params: &LandingMatrixQuery,
    x_feature: LandingMatrixXFeature,
) {
    let mut first = true;
    if let Some(months) = &params.months {
        if params.active_filter != ActiveLandingFilter::Date
            && x_feature != LandingMatrixXFeature::Date
        {
            if first {
                first = false;
                query.push_str("where ");
            } else {
                query.push_str("and ");
            }
            query.push_str(&format!("matrix_month_bucket = ANY ({:?}) ", months));
        }
    }
    if let Some(catch_locations) = &params.catch_locations {
        if params.active_filter != ActiveLandingFilter::Date
            && x_feature != LandingMatrixXFeature::Date
        {
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
            query.push_str(&format!("catch_location = ANY ([{filter}]) ",));
        }
    }
    if let Some(gear_group_ids) = &params.gear_group_ids {
        if params.active_filter != ActiveLandingFilter::GearGroup
            && x_feature != LandingMatrixXFeature::GearGroup
        {
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
    }
    if let Some(species_group_ids) = &params.species_group_ids {
        if params.active_filter != ActiveLandingFilter::SpeciesGroup
            && x_feature != LandingMatrixXFeature::SpeciesGroup
        {
            if first {
                first = false;
                query.push_str("where ");
            } else {
                query.push_str("and ");
            }
            query.push_str(&format!(
                "species_group_id = ANY ({:?}) ",
                species_group_ids
                    .iter()
                    .map(|v| *v as i32)
                    .collect::<Vec<i32>>()
            ));
        }
    }
    if let Some(vessel_length_groups) = &params.vessel_length_groups {
        if params.active_filter != ActiveLandingFilter::VesselLength
            && x_feature != LandingMatrixXFeature::VesselLength
        {
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
fn push_haul_where_statements(
    query: &mut String,
    params: &HaulsMatrixQuery,
    x_feature: HaulMatrixXFeature,
) {
    let mut first = true;
    if let Some(months) = &params.months {
        if params.active_filter != ActiveHaulsFilter::Date && x_feature != HaulMatrixXFeature::Date
        {
            if first {
                first = false;
                query.push_str("where ");
            } else {
                query.push_str("and ");
            }
            query.push_str(&format!("matrix_month_bucket = ANY ({:?}) ", months));
        }
    }
    if let Some(catch_locations) = &params.catch_locations {
        if params.active_filter != ActiveHaulsFilter::Date && x_feature != HaulMatrixXFeature::Date
        {
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
            query.push_str(&format!("catch_location = ANY ([{filter}]) ",));
        }
    }
    if let Some(gear_group_ids) = &params.gear_group_ids {
        if params.active_filter != ActiveHaulsFilter::GearGroup
            && x_feature != HaulMatrixXFeature::GearGroup
        {
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
    }
    if let Some(species_group_ids) = &params.species_group_ids {
        if params.active_filter != ActiveHaulsFilter::SpeciesGroup
            && x_feature != HaulMatrixXFeature::SpeciesGroup
        {
            if first {
                first = false;
                query.push_str("where ");
            } else {
                query.push_str("and ");
            }
            query.push_str(&format!(
                "species_group_id = ANY ({:?}) ",
                species_group_ids
                    .iter()
                    .map(|v| *v as i32)
                    .collect::<Vec<i32>>()
            ));
        }
    }
    if let Some(vessel_length_groups) = &params.vessel_length_groups {
        if params.active_filter != ActiveHaulsFilter::VesselLength
            && x_feature != HaulMatrixXFeature::VesselLength
        {
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
    RefreshCommunication,
}

impl std::fmt::Display for DuckdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuckdbError::Connection => f.write_str("failed to establish connection with duckdb"),
            DuckdbError::Query => f.write_str("failed to perfom a query"),
            DuckdbError::Conversion => f.write_str("failed to convert output of query"),
            DuckdbError::RefreshCommunication => {
                f.write_str("failed to communicate with the refresh task")
            }
        }
    }
}

impl Context for DuckdbError {}
