use std::{collections::BTreeMap, time::Duration};

use error_stack::{Result, ResultExt};
use futures::future::{BoxFuture, FutureExt};
use kyogre_core::{Ordering, TripDetailed, TripId, TripSorting, TripsQuery};
use meilisearch_sdk::{Error, ErrorCode, Index, Selectors, TaskInfo};
use serde::Deserialize;
use tracing::{event, Level};

use crate::{error::MeilisearchError, MeilisearchAdapter};

mod model;

pub use self::model::*;

impl MeilisearchAdapter {
    pub(crate) async fn trips_impl(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<Vec<TripDetailed>, MeilisearchError> {
        let mut filter = Vec::with_capacity(9);

        if let Some(ids) = query.fiskeridir_vessel_ids {
            filter.push(format!(
                "fiskeridir_vessel_id IN [{}]",
                join_comma_fn(ids, |i| i.0)
            ));
        }
        if let Some(groups) = query.vessel_length_groups {
            filter.push(format!(
                "fiskeridir_length_group_id IN [{}]",
                join_comma_fn(groups, |g| g as i32)
            ));
        }
        if let Some(start_date) = query.start_date {
            filter.push(format!("start >= {}", start_date.timestamp_millis()));
        }
        if let Some(end_date) = query.end_date {
            filter.push(format!("end <= {}", end_date.timestamp_millis()));
        }
        if let Some(min_weight) = query.min_weight {
            filter.push(format!("total_living_weight >= {}", min_weight));
        }
        if let Some(max_weight) = query.max_weight {
            filter.push(format!("total_living_weight <= {}", max_weight));
        }
        if let Some(gears) = query.gear_group_ids {
            filter.push(format!(
                "gear_group_ids IN [{}]",
                join_comma_fn(gears, |g| g as i32)
            ));
        }
        if let Some(species) = query.species_group_ids {
            filter.push(format!(
                "species_group_ids IN [{}]",
                join_comma_fn(species, |s| s as i32)
            ));
        }
        if let Some(ids) = query.delivery_points {
            filter.push(format!("delivery_point_ids IN [{}]", join_comma(ids)));
        }

        let filter = filter.iter().map(|f| f.as_str()).collect();

        let result = self
            .trips_index()
            .search()
            .with_array_filter(filter)
            .with_sort(&[&format!(
                "{}:{}",
                match query.sorting {
                    TripSorting::StopDate => "end",
                    TripSorting::Weight => "total_living_weight",
                },
                match query.ordering {
                    Ordering::Asc => "asc",
                    Ordering::Desc => "desc",
                }
            )])
            .with_limit(query.pagination.limit() as usize)
            .with_offset(query.pagination.offset() as usize)
            .execute::<Trip>()
            .await
            .change_context(MeilisearchError::Query)?;

        let trips = result
            .hits
            .into_iter()
            .map(|h| h.result.try_to_trip_detailed(read_fishing_facility))
            .collect::<Result<_, _>>()
            .change_context(MeilisearchError::Query)?;

        Ok(trips)
    }

    pub(crate) async fn all_trip_versions(
        &self,
    ) -> Result<BTreeMap<TripId, i64>, MeilisearchError> {
        #[derive(Deserialize)]
        struct TripId_ {
            trip_id: i64,
            cache_version: i64,
        }

        let result = self
            .trips_index()
            .search()
            .with_attributes_to_retrieve(Selectors::Some(&["trip_id", "cache_version"]))
            .with_limit(usize::MAX)
            .execute::<TripId_>()
            .await
            .change_context(MeilisearchError::Query)?;

        let result = result
            .hits
            .into_iter()
            .map(|h| (TripId(h.result.trip_id), h.result.cache_version))
            .collect();

        Ok(result)
    }

    pub(crate) async fn refresh_trips(&self) -> Result<(), MeilisearchError> {
        let cache_versions = self.all_trip_versions().await?;

        let source_versions = self
            .source
            .all_trip_versions()
            .await
            .change_context(MeilisearchError::Source)?
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        let to_delete = cache_versions
            .keys()
            .copied()
            .filter(|i| !source_versions.contains_key(i))
            .map(|id| id.0)
            .collect::<Vec<_>>();

        let to_insert = source_versions
            .into_iter()
            .filter(|(id, sv)| cache_versions.get(id).map(|cv| sv > cv).unwrap_or(true))
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        let index = self.trips_index();

        let mut tasks = Vec::new();

        if !to_delete.is_empty() {
            let task = index
                .delete_documents(&to_delete)
                .await
                .change_context(MeilisearchError::Delete)?;
            tasks.push(task);
        }

        fn add_trips<'a>(
            index: &'a Index,
            tasks: &'a mut Vec<TaskInfo>,
            trips: &'a [Trip],
        ) -> BoxFuture<'a, ()> {
            async move {
                match index.add_documents(trips, Some(Trip::primary_key())).await {
                    Ok(task) => tasks.push(task),
                    Err(e) => match e {
                        Error::Meilisearch(meilisearch_sdk::MeilisearchError {
                            error_code: ErrorCode::PayloadTooLarge,
                            ..
                        }) => {
                            if trips.len() == 1 {
                                event!(
                                    Level::ERROR,
                                    "trip {} is too large to insert into meilisearch",
                                    trips[0].trip_id.0,
                                );
                            } else {
                                event!(
                                    Level::WARN,
                                    "Insert payload too large with {} trips",
                                    trips.len(),
                                );

                                let (left, right) = trips.split_at(trips.len() / 2);
                                add_trips(index, tasks, left).await;
                                add_trips(index, tasks, right).await;
                            }
                        }
                        _ => event!(Level::ERROR, "failed to insert trips, error: {e:?}"),
                    },
                }
            }
            .boxed()
        }

        for ids in to_insert.chunks(20_000) {
            let trips = self
                .source
                .trips(ids)
                .await
                .change_context(MeilisearchError::Source)?
                .into_iter()
                .map(Trip::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            add_trips(&index, &mut tasks, &trips).await;
        }

        for task in tasks {
            let task = task
                .wait_for_completion(
                    &self.client,
                    Some(Duration::from_secs(30)),
                    // We insert a lot of trips, so use a decently large timeout.
                    Some(Duration::from_secs(60 * 60)),
                )
                .await
                .change_context(MeilisearchError::Query)?;

            if !task.is_success() {
                event!(
                    Level::ERROR,
                    "failed to process trips insert/delete task: {task:?}"
                );
            }
        }

        Ok(())
    }
}

fn join_comma<T: ToString>(values: Vec<T>) -> String {
    values
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}
fn join_comma_fn<T, S: ToString, F: Fn(T) -> S>(values: Vec<T>, closure: F) -> String {
    values
        .into_iter()
        .map(|v| closure(v).to_string())
        .collect::<Vec<_>>()
        .join(",")
}
