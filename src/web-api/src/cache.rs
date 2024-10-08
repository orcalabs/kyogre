use crate::{Cache, Meilisearch};
use async_trait::async_trait;
use duckdb_rs::Client;
use fiskeridir_rs::LandingId;
use kyogre_core::{
    CoreResult, HaulId, HaulsMatrixQuery, HaulsQuery, LandingMatrixQuery, LandingsQuery,
    MatrixCacheOutbound, MeilisearchOutbound, TripDetailed, TripsQuery,
};
use meilisearch::MeilisearchAdapter;
use postgres::PostgresAdapter;
use serde::Deserialize;
use tracing::error;

// Used to trigger api errors when testing cache implementations
#[derive(Copy, Clone, Debug, Deserialize)]
pub enum CacheErrorMode {
    Propagate,
    Log,
}

#[derive(Clone)]
pub struct MeilesearchCache {
    inner: MeilisearchAdapter<PostgresAdapter>,
    error_mode: CacheErrorMode,
}

#[derive(Clone)]
pub struct MatrixCache {
    inner: Client,
    error_mode: CacheErrorMode,
}

impl MeilesearchCache {
    pub fn new(
        inner: MeilisearchAdapter<PostgresAdapter>,
        error_mode: CacheErrorMode,
    ) -> MeilesearchCache {
        MeilesearchCache { inner, error_mode }
    }
}

impl MatrixCache {
    pub fn new(inner: Client, error_mode: CacheErrorMode) -> MatrixCache {
        MatrixCache { inner, error_mode }
    }
}

impl Cache for MatrixCache {}
impl Meilisearch for MeilesearchCache {}

#[async_trait]
impl MatrixCacheOutbound for MatrixCache {
    async fn landing_matrix(
        &self,
        query: &LandingMatrixQuery,
    ) -> CoreResult<Option<kyogre_core::LandingMatrix>> {
        match self.inner.landing_matrix(query).await {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve landings matrix: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(None),
                }
            }
        }
    }
    async fn hauls_matrix(
        &self,
        query: &HaulsMatrixQuery,
    ) -> CoreResult<Option<kyogre_core::HaulsMatrix>> {
        match self.inner.hauls_matrix(query).await {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve hauls matrix: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(None),
                }
            }
        }
    }
}

#[async_trait]
impl MeilisearchOutbound for MeilesearchCache {
    async fn trips(
        &self,
        query: &TripsQuery,
        read_fishing_facility: bool,
    ) -> CoreResult<Vec<TripDetailed>> {
        match self.inner.trips(query, read_fishing_facility).await {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve hauls from meilisearch: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(vec![]),
                }
            }
        }
    }
    async fn trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>> {
        match self
            .inner
            .trip_of_haul(haul_id, read_fishing_facility)
            .await
        {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve trip_of_haul from meilisearch: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(None),
                }
            }
        }
    }
    async fn trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>> {
        match self
            .inner
            .trip_of_landing(landing_id, read_fishing_facility)
            .await
        {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve trip_of_landing from meilisearch: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(None),
                }
            }
        }
    }
    async fn hauls(&self, query: &HaulsQuery) -> CoreResult<Vec<kyogre_core::Haul>> {
        match self.inner.hauls(query).await {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve hauls from meilisearch: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(vec![]),
                }
            }
        }
    }
    async fn landings(&self, query: &LandingsQuery) -> CoreResult<Vec<kyogre_core::Landing>> {
        match self.inner.landings(query).await {
            Ok(v) => Ok(v),
            Err(e) => {
                error!("failed to retrieve landings from meilisearch: {e:?}");
                match self.error_mode {
                    CacheErrorMode::Propagate => Err(e),
                    CacheErrorMode::Log => Ok(vec![]),
                }
            }
        }
    }
}
