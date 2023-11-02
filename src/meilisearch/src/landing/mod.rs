use error_stack::{Result, ResultExt};
use kyogre_core::{LandingsQuery, LandingsSorting, Ordering};

use crate::{
    create_ranges_filter, error::MeilisearchError, join_comma, join_comma_fn, MeilisearchAdapter,
};

mod model;

pub use model::*;

impl MeilisearchAdapter {
    pub(crate) async fn landings_impl(
        &self,
        query: LandingsQuery,
    ) -> Result<Vec<kyogre_core::Landing>, MeilisearchError> {
        let mut filter = Vec::with_capacity(9);

        if let Some(ids) = query.vessel_ids {
            filter.push(format!(
                "fiskeridir_vessel_id IN [{}]",
                join_comma_fn(ids, |i| i.0)
            ));
        }
        if let Some(ranges) = query.vessel_length_ranges {
            filter.push(create_ranges_filter(
                ranges,
                "vessel_length",
                "vessel_length",
            ));
        }
        if let Some(ranges) = query.ranges {
            filter.push(create_ranges_filter(
                ranges,
                "landing_timestamp",
                "vessel_length",
            ));
        }
        if let Some(ids) = query.gear_group_ids {
            filter.push(format!(
                "gear_group_ids IN [{}]",
                join_comma_fn(ids, |g| g as i32)
            ));
        }
        if let Some(ids) = query.species_group_ids {
            filter.push(format!(
                "species_group_ids IN [{}]",
                join_comma_fn(ids, |s| s as i32)
            ));
        }
        if let Some(locs) = query.catch_locations {
            filter.push(format!("catch_locations IN [{}]", join_comma(locs)));
        }

        let filter = filter.iter().map(|f| f.as_str()).collect();

        let sort_string = query.sorting.map(|sorting| {
            format!(
                "{}:{}",
                match sorting {
                    LandingsSorting::LandingTimestamp => "landing_timestamp",
                    LandingsSorting::LivingWeight => "living_weight",
                },
                match query.ordering.unwrap_or_default() {
                    Ordering::Asc => "asc",
                    Ordering::Desc => "desc",
                }
            )
        });
        let sort = sort_string
            .as_ref()
            .map(|s| vec![s.as_str()])
            .unwrap_or_default();

        let result = self
            .landings_index()
            .search()
            .with_array_filter(filter)
            .with_limit(usize::MAX)
            .with_sort(&sort)
            .execute::<Landing>()
            .await
            .change_context(MeilisearchError::Query)?;

        let landings = result
            .hits
            .into_iter()
            .map(|h| h.result.try_into())
            .collect::<Result<_, _>>()
            .change_context(MeilisearchError::Query)?;

        Ok(landings)
    }
}
