#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use error::Result;
use fiskeridir_rs::LandingId;
use indexable::Indexable;
use kyogre_core::{
    retry, CoreResult, HaulId, HaulsQuery, LandingsQuery, MeilisearchOutbound, MeilisearchSource,
    TripDetailed, TripsQuery,
};
use meilisearch_sdk::client::Client;
use std::time::Duration;

mod error;
mod haul;
mod indexable;
mod landing;
mod query;
pub mod settings;
mod trip;
mod utils;

use haul::*;
use landing::*;
pub use settings::Settings;
use strum::{AsRefStr, EnumIter, EnumString, IntoEnumIterator};
use tracing::{error, instrument};
use trip::*;

#[derive(Clone)]
pub struct MeilisearchAdapter<T> {
    pub client: Client,
    pub source: T,
    pub refresh_timeout: Duration,
    pub index_suffix: Option<String>,
}

#[derive(EnumIter, strum::Display, AsRefStr, EnumString)]
#[strum(serialize_all = "snake_case")]
enum CacheIndex {
    Trips,
    Hauls,
    Landings,
}

impl<T> MeilisearchAdapter<T> {
    pub fn new(settings: &Settings, source: T) -> Self {
        Self {
            client: Client::new(&settings.host, Some(&settings.api_key)).unwrap(),
            source,
            refresh_timeout: settings
                .refresh_timeout
                .unwrap_or_else(|| Duration::from_secs(600)),
            index_suffix: settings.index_suffix.clone(),
        }
    }
}

impl<T: Sync> MeilisearchAdapter<T> {
    pub async fn create_indexes(&self) -> Result<()> {
        for c in CacheIndex::iter() {
            match c {
                CacheIndex::Trips => Trip::create_index(self).await,
                CacheIndex::Hauls => Haul::create_index(self).await,
                CacheIndex::Landings => Landing::create_index(self).await,
            }?;
        }

        Ok(())
    }

    pub async fn cleanup(&self) -> Result<()> {
        for c in CacheIndex::iter() {
            match c {
                CacheIndex::Trips => Trip::cleanup(self).await,
                CacheIndex::Hauls => Haul::cleanup(self).await,
                CacheIndex::Landings => Landing::cleanup(self).await,
            }?;
        }

        Ok(())
    }
}

impl<T: MeilisearchSource> MeilisearchAdapter<T> {
    #[instrument(name = "refresh_meilisearch", skip(self))]
    pub async fn refresh(&self) -> Result<()> {
        for c in CacheIndex::iter() {
            match c {
                CacheIndex::Trips => Trip::refresh(self).await,
                CacheIndex::Hauls => Haul::refresh(self).await,
                CacheIndex::Landings => Landing::refresh(self).await,
            }?;
        }

        Ok(())
    }

    pub async fn run(self) -> ! {
        loop {
            if let Err(e) = self.refresh().await {
                error!("meilisearch `refresh` failed with error: {e:?}");
            }

            tokio::time::sleep(self.refresh_timeout).await;
        }
    }
}

#[async_trait]
impl<T: Send + Sync> MeilisearchOutbound for MeilisearchAdapter<T> {
    async fn trips(
        &self,
        query: &TripsQuery,
        read_fishing_facility: bool,
    ) -> CoreResult<Vec<TripDetailed>> {
        Ok(retry(|| self.trips_impl(query.clone(), read_fishing_facility)).await?)
    }
    async fn trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>> {
        Ok(retry(|| self.trip_of_haul_impl(haul_id, read_fishing_facility)).await?)
    }
    async fn trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> CoreResult<Option<TripDetailed>> {
        Ok(retry(|| self.trip_of_landing_impl(landing_id, read_fishing_facility)).await?)
    }
    async fn hauls(&self, query: &HaulsQuery) -> CoreResult<Vec<kyogre_core::Haul>> {
        Ok(retry(|| self.hauls_impl(query.clone())).await?)
    }
    async fn landings(&self, query: &LandingsQuery) -> CoreResult<Vec<kyogre_core::Landing>> {
        Ok(retry(|| self.landings_impl(query.clone())).await?)
    }
}
