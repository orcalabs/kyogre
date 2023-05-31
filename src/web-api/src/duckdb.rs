use duckdb::DuckdbConnectionManager;
use error_stack::{Context, IntoReport, Result, ResultExt};
use kyogre_core::{CacheOutboundPort, HaulsMatrix, HaulsMatrixQuery, QueryError};
use orca_core::PsqlSettings;
use serde::Deserialize;

use crate::Cache;

#[allow(dead_code)]
#[derive(Clone)]
pub struct DuckdbAdapter {
    pool: r2d2::Pool<DuckdbConnectionManager>,
}

#[derive(Debug, Deserialize)]
pub struct DuckdbSettings {
    pub max_memory_mb: Option<u32>,
    pub max_connections: u32,
}

impl DuckdbAdapter {
    pub fn new(
        settings: &DuckdbSettings,
        _postgres_settings: &PsqlSettings,
    ) -> Result<DuckdbAdapter, DuckdbError> {
        let manager = DuckdbConnectionManager::memory()
            .into_report()
            .change_context(DuckdbError::Connection)?;
        let pool = r2d2::Pool::builder()
            .max_size(settings.max_connections)
            .build(manager)
            .into_report()
            .change_context(DuckdbError::Connection)?;

        Ok(DuckdbAdapter { pool })
    }
}

impl CacheOutboundPort for DuckdbAdapter {
    fn hauls_matrix(&self, _query: &HaulsMatrixQuery) -> Result<Option<HaulsMatrix>, QueryError> {
        Ok(None)
    }
}

impl Cache for DuckdbAdapter {}

#[derive(Debug)]
pub enum DuckdbError {
    Connection,
}

impl std::fmt::Display for DuckdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuckdbError::Connection => f.write_str("failed to establish connection with duckdb"),
        }
    }
}

impl Context for DuckdbError {}
