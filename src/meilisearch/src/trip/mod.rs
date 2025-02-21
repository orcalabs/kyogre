use kyogre_core::{Pagination, TripDetailed, Trips, TripsQuery};

use crate::{MeilisearchAdapter, error::Result, indexable::Indexable, query::Query};

mod filter;
mod model;

pub use filter::*;
pub use model::*;

impl<T> MeilisearchAdapter<T> {
    pub(crate) async fn trips_impl(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<Vec<TripDetailed>> {
        let query = Query::<TripFilter, TripSort, Pagination<Trips>>::from(query);

        let pagination = query.pagination;

        let sort_string = query.sort_str();
        let sort = vec![sort_string.as_str()];

        let filter = query.filter_strs()?;
        let filter = filter.iter().map(|f| f.as_str()).collect();

        let result = Trip::index(self)
            .search()
            .with_array_filter(filter)
            .with_sort(&sort)
            .with_limit(pagination.limit() as usize)
            .with_offset(pagination.offset() as usize)
            .execute::<Trip>()
            .await?;

        let trips = result
            .hits
            .into_iter()
            .map(|h| h.result.try_to_trip_detailed(read_fishing_facility))
            .collect::<Result<_>>()?;

        Ok(trips)
    }
}
