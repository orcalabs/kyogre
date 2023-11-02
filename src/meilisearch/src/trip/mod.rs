use error_stack::{Result, ResultExt};
use kyogre_core::{Ordering, TripDetailed, TripSorting, TripsQuery};

use crate::{error::MeilisearchError, join_comma, join_comma_fn, to_nanos, MeilisearchAdapter};

mod model;

pub use model::*;

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
            filter.push(format!("start >= {}", to_nanos(start_date)?));
        }
        if let Some(end_date) = query.end_date {
            filter.push(format!("end <= {}", to_nanos(end_date)?));
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
}
