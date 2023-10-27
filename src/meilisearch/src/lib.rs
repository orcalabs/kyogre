#![deny(warnings)]
#![deny(rust_2018_idioms)]

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use error::MeilisearchError;
use error_stack::{Result, ResultExt};
use kyogre_core::{MeilisearchOutbound, MeilisearchSource, QueryError, TripDetailed, TripsQuery};
use meilisearch_sdk::{Client, Index};

mod error;
pub mod settings;
mod trip;

pub use settings::Settings;
use tracing::{event, instrument, Level};
use trip::*;

#[derive(Clone)]
pub struct MeilisearchAdapter {
    pub client: Client,
    pub source: Arc<dyn MeilisearchSource>,
}

impl MeilisearchAdapter {
    pub fn new(settings: &Settings, source: Arc<dyn MeilisearchSource>) -> Self {
        Self {
            client: Client::new(&settings.host, Some(&settings.api_key)),
            source,
        }
    }

    pub(crate) fn trips_index(&self) -> Index {
        self.client.index(Trip::index_name())
    }

    pub async fn create_indexes(&self) -> Result<(), MeilisearchError> {
        Trip::create_index(&self.client).await?;
        Ok(())
    }

    #[instrument(name = "refresh_meilisearch", skip(self))]
    async fn refresh(&self) -> Result<(), MeilisearchError> {
        self.refresh_trips().await?;
        Ok(())
    }

    pub async fn run(self) {
        loop {
            if let Err(e) = self.refresh().await {
                event!(
                    Level::ERROR,
                    "meilisearch `refresh` failed with error: {e:?}"
                );
            }

            tokio::time::sleep(Duration::from_secs(60 * 10)).await;
        }
    }
}

#[async_trait]
impl MeilisearchOutbound for MeilisearchAdapter {
    async fn trips(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<Vec<TripDetailed>, QueryError> {
        self.trips_impl(query, read_fishing_facility)
            .await
            .change_context(QueryError)
    }
}
