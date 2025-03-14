use std::{
    collections::BTreeMap,
    fmt::{Debug, Display},
    time::Duration,
};

use async_trait::async_trait;
use futures::{FutureExt, future::BoxFuture};
use kyogre_core::{MeilisearchSource, running_in_test};
use meilisearch_sdk::{
    errors::ErrorCode, indexes::Index, search::Selectors, settings::PaginationSetting,
    task_info::TaskInfo, tasks::Task,
};
use serde::{Serialize, de::DeserializeOwned};
use strum::IntoEnumIterator;
use tracing::{error, info, warn};

use crate::{
    CacheIndex, MeilisearchAdapter,
    error::{Result, error::TaskSnafu},
};

pub trait IdVersion {
    type Id;

    fn id(&self) -> Self::Id;
    fn version(&self) -> i64;
}
pub trait Id {
    type Id: Display;

    fn id(&self) -> Self::Id;
}

#[async_trait]
pub trait Indexable {
    type Id: Clone + Eq + Ord + Debug + Display + Serialize + Send + Sync;
    type Item: Id + Serialize + Debug + Send + Sync;
    type IdVersion: IdVersion<Id = Self::Id> + DeserializeOwned + 'static + Send + Sync;
    type FilterableAttributes: IntoEnumIterator + Display;
    type SortableAttributes: IntoEnumIterator + Display;

    fn cache_index() -> CacheIndex;
    fn primary_key() -> &'static str;
    fn chunk_size() -> usize;
    async fn source_versions<T: MeilisearchSource>(source: &T) -> Result<Vec<(Self::Id, i64)>>;
    async fn items_by_ids<T: MeilisearchSource>(
        source: &T,
        ids: &[Self::Id],
    ) -> Result<Vec<Self::Item>>;

    fn index<T>(adapter: &MeilisearchAdapter<T>) -> Index {
        let uid = match &adapter.index_suffix {
            Some(suffix) => format!("{}_{}", Self::cache_index(), suffix),
            None => Self::cache_index().to_string(),
        };
        adapter.client.index(uid)
    }

    async fn create_index<T: Sync>(adapter: &MeilisearchAdapter<T>) -> Result<()> {
        let settings = meilisearch_sdk::settings::Settings::new()
            .with_searchable_attributes(Vec::<String>::new())
            .with_ranking_rules(["sort"])
            .with_filterable_attributes(Self::FilterableAttributes::iter().map(|d| format!("{d}")))
            .with_sortable_attributes(Self::SortableAttributes::iter().map(|d| format!("{d}")))
            .with_pagination(PaginationSetting {
                max_total_hits: usize::MAX,
            });

        let task = Self::index(adapter)
            .set_settings(&settings)
            .await?
            .wait_for_completion(&adapter.client, None, Some(Duration::from_secs(60 * 10)))
            .await?;

        if !task.is_success() {
            return TaskSnafu { task }.fail();
        }

        Ok(())
    }

    async fn refresh<T: MeilisearchSource>(adapter: &MeilisearchAdapter<T>) -> Result<()> {
        let index = Self::index(adapter);

        let cache_versions = Self::all_versions(&index).await?;

        let source_versions = Self::source_versions(&adapter.source)
            .await?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

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

        info!("{} to delete: {}", index.uid, to_delete.len());
        info!("{} to insert: {}", index.uid, to_insert.len());

        let mut tasks = Vec::new();

        // Deleting too many items causes the payload to be too large,
        // so split into multiple requests
        for ids in to_delete.chunks(50_000) {
            let task = index.delete_documents(ids).await?;
            tasks.push(task);
        }

        for ids in to_insert.chunks(Self::chunk_size()) {
            let items = Self::items_by_ids(&adapter.source, ids).await?;
            Self::add_items(&index, &mut tasks, &items).await;
        }

        let interval = if running_in_test() {
            None
        } else {
            Some(Duration::from_secs(30))
        };

        for task in tasks {
            let task = task
                .wait_for_completion(
                    &adapter.client,
                    interval,
                    // We insert a lot of items, so use a decently large timeout.
                    Some(Duration::from_secs(60 * 60)),
                )
                .await?;

            if !task.is_success() {
                error!("insert/delete task did not succeed: {task:?}");
            }
        }

        Ok(())
    }

    async fn all_versions(index: &Index) -> Result<BTreeMap<Self::Id, i64>> {
        let primary_key = Self::primary_key();

        let result = index
            .search()
            .with_attributes_to_retrieve(Selectors::Some(&[primary_key, "cache_version"]))
            .with_limit(usize::MAX)
            .execute::<Self::IdVersion>()
            .await?;

        let result = result
            .hits
            .into_iter()
            .map(|h| (h.result.id(), h.result.version()))
            .collect();

        Ok(result)
    }

    fn add_items<'a>(
        index: &'a Index,
        tasks: &'a mut Vec<TaskInfo>,
        items: &'a [Self::Item],
    ) -> BoxFuture<'a, ()> {
        use meilisearch_sdk::errors::{Error, ErrorCode, MeilisearchError};

        async move {
            match index.add_documents(items, Some(Self::primary_key())).await {
                Ok(task) => tasks.push(task),
                Err(e) => match e {
                    Error::Meilisearch(MeilisearchError {
                        error_code: ErrorCode::PayloadTooLarge,
                        ..
                    }) => {
                        if items.len() == 1 {
                            error!(
                                "item with {} {} is too large to insert into meilisearch",
                                Self::primary_key(),
                                items[0].id(),
                            );
                        } else {
                            warn!(
                                "insert payload too large with {} {}",
                                items.len(),
                                index.uid,
                            );

                            let (left, right) = items.split_at(items.len() / 2);
                            Self::add_items(index, tasks, left).await;
                            Self::add_items(index, tasks, right).await;
                        }
                    }
                    _ => error!("failed to insert items, error: {e:?}"),
                },
            }
        }
        .boxed()
    }

    async fn cleanup<T: Sync>(adapter: &MeilisearchAdapter<T>) -> Result<()> {
        let task = Self::index(adapter)
            .delete()
            .await?
            .wait_for_completion(&adapter.client, None, Some(Duration::from_secs(60)))
            .await?;

        match &task {
            // Should never happen as we wait for completion
            Task::Enqueued { .. } | Task::Processing { .. } => {
                unreachable!();
            }
            Task::Failed { content } => match content.error.error_code {
                ErrorCode::IndexNotFound => Ok(()),
                _ => TaskSnafu { task }.fail(),
            },
            Task::Succeeded { .. } => Ok(()),
        }
    }
}
