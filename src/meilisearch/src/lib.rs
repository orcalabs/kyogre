#![deny(warnings)]
#![deny(rust_2018_idioms)]

use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    ops::Bound,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use error::MeilisearchError;
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::LandingId;
use futures::{future::BoxFuture, FutureExt};
use kyogre_core::{
    running_in_test, HaulId, HaulsQuery, LandingsQuery, MeilisearchOutbound, MeilisearchSource,
    QueryError, Range, TripDetailed, TripsQuery,
};
use meilisearch_sdk::{Client, Index, Selectors, TaskInfo};

mod error;
mod haul;
mod landing;
pub mod settings;
mod trip;

use haul::*;
use landing::*;
use serde::{de::DeserializeOwned, Serialize};
pub use settings::Settings;
use tracing::{event, instrument, Level};
use trip::*;

#[derive(Clone)]
pub struct MeilisearchAdapter {
    pub client: Client,
    pub source: Arc<dyn MeilisearchSource>,
    pub index_suffix: String,
}

impl MeilisearchAdapter {
    pub fn new(settings: &Settings, source: Arc<dyn MeilisearchSource>) -> Self {
        Self {
            client: Client::new(&settings.host, Some(&settings.api_key)),
            source,
            index_suffix: settings.index_suffix.clone().unwrap_or_default(),
        }
    }

    pub(crate) fn trips_index(&self) -> Index {
        let index_name = format!("trips{}", self.index_suffix);
        self.client.index(index_name)
    }
    pub(crate) fn hauls_index(&self) -> Index {
        let index_name = format!("hauls{}", self.index_suffix);
        self.client.index(index_name)
    }
    pub(crate) fn landings_index(&self) -> Index {
        let index_name = format!("landings{}", self.index_suffix);
        self.client.index(index_name)
    }

    pub async fn create_indexes(&self) -> Result<(), MeilisearchError> {
        Trip::create_index(self).await?;
        Haul::create_index(self).await?;
        Landing::create_index(self).await?;
        Ok(())
    }

    pub async fn cleanup(&self) -> Result<(), MeilisearchError> {
        Trip::cleanup(self).await?;
        Haul::cleanup(self).await?;
        Landing::cleanup(self).await?;
        Ok(())
    }

    #[instrument(name = "refresh_meilisearch", skip(self))]
    pub async fn refresh(&self) -> Result<(), MeilisearchError> {
        Trip::refresh(self).await?;
        Haul::refresh(self).await?;
        Landing::refresh(self).await?;
        Ok(())
    }

    pub async fn run(self) -> ! {
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
    async fn trip_of_haul(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError> {
        self.trip_of_haul_impl(haul_id, read_fishing_facility)
            .await
            .change_context(QueryError)
    }
    async fn trip_of_landing(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, QueryError> {
        self.trip_of_landing_impl(landing_id, read_fishing_facility)
            .await
            .change_context(QueryError)
    }
    async fn hauls(&self, query: HaulsQuery) -> Result<Vec<kyogre_core::Haul>, QueryError> {
        self.hauls_impl(query).await.change_context(QueryError)
    }
    async fn landings(
        &self,
        query: LandingsQuery,
    ) -> Result<Vec<kyogre_core::Landing>, QueryError> {
        self.landings_impl(query).await.change_context(QueryError)
    }
}

pub(crate) trait IdVersion {
    type Id;

    fn id(&self) -> Self::Id;
    fn version(&self) -> i64;
}
pub(crate) trait Id {
    type Id: Display;

    fn id(&self) -> Self::Id;
}

#[async_trait]
pub(crate) trait Indexable {
    type Id: Clone + Eq + Ord + Debug + Display + Serialize + Sync;
    type Item: Id + Serialize + Debug + Sync;
    type IdVersion: IdVersion<Id = Self::Id> + DeserializeOwned + 'static;

    fn index(adapter: &MeilisearchAdapter) -> Index;
    fn primary_key() -> &'static str;
    async fn refresh(adapter: &MeilisearchAdapter) -> Result<(), MeilisearchError>;

    async fn all_versions(index: &Index) -> Result<BTreeMap<Self::Id, i64>, MeilisearchError> {
        let primary_key = Self::primary_key();

        let result = index
            .search()
            .with_attributes_to_retrieve(Selectors::Some(&[primary_key, "cache_version"]))
            .with_limit(usize::MAX)
            .execute::<Self::IdVersion>()
            .await
            .change_context(MeilisearchError::Query)?;

        let result = result
            .hits
            .into_iter()
            .map(|h| (h.result.id(), h.result.version()))
            .collect();

        Ok(result)
    }

    fn to_insert_and_delete(
        cache_versions: BTreeMap<Self::Id, i64>,
        source_versions: BTreeMap<Self::Id, i64>,
    ) -> (Vec<Self::Id>, Vec<Self::Id>) {
        let to_delete = cache_versions
            .keys()
            .filter(|i| !source_versions.contains_key(i))
            .cloned()
            .collect::<Vec<_>>();

        let to_insert = source_versions
            .into_iter()
            .filter(|(id, sv)| cache_versions.get(id).map(|cv| sv > cv).unwrap_or(true))
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        (to_insert, to_delete)
    }

    async fn delete_items(
        index: &Index,
        ids: &[Self::Id],
    ) -> Result<Option<TaskInfo>, MeilisearchError> {
        if ids.is_empty() {
            Ok(None)
        } else {
            index
                .delete_documents(ids)
                .await
                .change_context(MeilisearchError::Delete)
                .map(Some)
        }
    }

    fn add_items<'a>(
        index: &'a Index,
        tasks: &'a mut Vec<TaskInfo>,
        items: &'a [Self::Item],
    ) -> BoxFuture<'a, ()> {
        use meilisearch_sdk::{Error, ErrorCode, MeilisearchError};

        async move {
            match index.add_documents(items, Some(Self::primary_key())).await {
                Ok(task) => tasks.push(task),
                Err(e) => match e {
                    Error::Meilisearch(MeilisearchError {
                        error_code: ErrorCode::PayloadTooLarge,
                        ..
                    }) => {
                        if items.len() == 1 {
                            event!(
                                Level::ERROR,
                                "item with {} {} is too large to insert into meilisearch",
                                Self::primary_key(),
                                items[0].id(),
                            );
                        } else {
                            event!(
                                Level::WARN,
                                "Insert payload too large with {} items",
                                items.len(),
                            );

                            let (left, right) = items.split_at(items.len() / 2);
                            Self::add_items(index, tasks, left).await;
                            Self::add_items(index, tasks, right).await;
                        }
                    }
                    _ => event!(Level::ERROR, "failed to insert items, error: {e:?}"),
                },
            }
        }
        .boxed()
    }

    async fn wait_for_completion(
        client: &Client,
        tasks: Vec<TaskInfo>,
    ) -> Result<(), MeilisearchError> {
        let interval = if running_in_test() {
            None
        } else {
            Some(Duration::from_secs(30))
        };

        for task in tasks {
            let task = task
                .wait_for_completion(
                    client,
                    interval,
                    // We insert a lot of items, so use a decently large timeout.
                    Some(Duration::from_secs(60 * 60)),
                )
                .await
                .change_context(MeilisearchError::Query)?;

            if !task.is_success() {
                event!(Level::ERROR, "insert/delete task did not succeed: {task:?}");
            }
        }

        Ok(())
    }

    async fn cleanup(adapter: &MeilisearchAdapter) -> Result<(), MeilisearchError> {
        let task = Self::index(adapter)
            .delete()
            .await
            .change_context(MeilisearchError::Delete)?
            .wait_for_completion(&adapter.client, None, Some(Duration::from_secs(60)))
            .await
            .change_context(MeilisearchError::Delete)?;

        if task.is_success() {
            Ok(())
        } else {
            Err(report!(MeilisearchError::Delete)
                .attach_printable(format!("failed to delete index: {task:?}")))
        }
    }
}

pub(crate) fn to_nanos(timestamp: DateTime<Utc>) -> Result<i64, MeilisearchError> {
    timestamp.timestamp_nanos_opt().ok_or_else(|| {
        report!(MeilisearchError::DataConversion).attach_printable(format!(
            "{} could not be converted to timestamp nanos",
            timestamp
        ))
    })
}

pub(crate) fn join_comma<T: ToString>(values: Vec<T>) -> String {
    values
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
pub(crate) fn join_comma_fn<T, S: ToString, F: Fn(T) -> S>(values: Vec<T>, closure: F) -> String {
    values
        .into_iter()
        .map(|v| closure(v).to_string())
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn create_ranges_filter<I, T, S>(ranges: I, attr1: S, attr2: S) -> String
where
    I: IntoIterator<Item = Range<T>>,
    T: Display,
    S: Display,
{
    ranges
        .into_iter()
        .flat_map(|range| {
            let start = match range.start {
                Bound::Included(v) => Some(format!("{attr1} >= {v}")),
                Bound::Excluded(v) => Some(format!("{attr1} > {v}")),
                Bound::Unbounded => None,
            };
            let end = match range.end {
                Bound::Included(v) => Some(format!("{attr2} <= {v}")),
                Bound::Excluded(v) => Some(format!("{attr2} < {v}")),
                Bound::Unbounded => None,
            };
            match (start, end) {
                (Some(a), Some(b)) => Some(format!("({a} AND {b})")),
                (Some(a), None) => Some(a),
                (None, Some(b)) => Some(b),
                (None, None) => None,
            }
        })
        .collect::<Vec<_>>()
        .join(" OR ")
}
