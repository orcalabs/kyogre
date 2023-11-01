use std::ops::Bound;

use error_stack::{Result, ResultExt};
use kyogre_core::{HaulsQuery, HaulsSorting, Ordering};

use crate::{error::MeilisearchError, join_comma, join_comma_fn, MeilisearchAdapter};

mod model;

pub use model::*;

impl MeilisearchAdapter {
    pub(crate) async fn hauls_impl(
        &self,
        query: HaulsQuery,
    ) -> Result<Vec<kyogre_core::Haul>, MeilisearchError> {
        let mut filter = Vec::with_capacity(9);

        if let Some(ids) = query.vessel_ids {
            filter.push(format!(
                "fiskeridir_vessel_id IN [{}]",
                join_comma_fn(ids, |i| i.0)
            ));
        }
        if let Some(ranges) = query.vessel_length_ranges {
            filter.push(
                ranges
                    .into_iter()
                    .flat_map(|range| {
                        let start = match range.start {
                            Bound::Included(v) => Some(format!("stop_timestamp >= {v}")),
                            Bound::Excluded(v) => Some(format!("stop_timestamp > {v}")),
                            Bound::Unbounded => None,
                        };
                        let end = match range.end {
                            Bound::Included(v) => Some(format!("start_timestamp <= {v}")),
                            Bound::Excluded(v) => Some(format!("start_timestamp < {v}")),
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
                    .join(" OR "),
            );
        }
        if let Some(value) = query.min_wind_speed {
            filter.push(format!("wind_speed >= {}", value));
        }
        if let Some(value) = query.max_wind_speed {
            filter.push(format!("wind_speed <= {}", value));
        }
        if let Some(value) = query.min_air_temperature {
            filter.push(format!("air_temperature >= {}", value));
        }
        if let Some(value) = query.max_air_temperature {
            filter.push(format!("air_temperature <= {}", value));
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
                    HaulsSorting::StartDate => "start_timestamp",
                    HaulsSorting::StopDate => "stop_timestamp",
                    HaulsSorting::Weight => "total_living_weight",
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
            .hauls_index()
            .search()
            .with_array_filter(filter)
            .with_limit(usize::MAX)
            .with_sort(&sort)
            .execute::<Haul>()
            .await
            .change_context(MeilisearchError::Query)?;

        let hauls = result
            .hits
            .into_iter()
            .map(|h| h.result.try_into())
            .collect::<Result<_, _>>()
            .change_context(MeilisearchError::Query)?;

        Ok(hauls)
    }
}
