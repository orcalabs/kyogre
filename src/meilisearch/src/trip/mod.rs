use error_stack::{Result, ResultExt};
use fiskeridir_rs::LandingId;
use kyogre_core::{HaulId, TripDetailed, TripsQuery};

use crate::{error::MeilisearchError, indexable::Indexable, MeilisearchAdapter};

mod model;
mod query;

pub use model::*;

use query::{Query, TripFilter, TripSort};

impl<T> MeilisearchAdapter<T> {
    pub(crate) async fn trips_impl(
        &self,
        query: TripsQuery,
        read_fishing_facility: bool,
    ) -> Result<Vec<TripDetailed>, MeilisearchError> {
        let pagination = query.pagination;
        let ordering = query.ordering;

        let sort = TripSort::from(query.sorting);
        let query = Query::from(query);

        let filter = query.filter_strs()?;
        let filter = filter.iter().map(|f| f.as_str()).collect();

        let result = Trip::index(self)
            .search()
            .with_array_filter(filter)
            .with_sort(&[&format!("{sort}:{ordering}")])
            .with_limit(pagination.limit() as usize)
            .with_offset(pagination.offset() as usize)
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

    pub(crate) async fn trip_of_haul_impl(
        &self,
        haul_id: &HaulId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, MeilisearchError> {
        let filter = TripFilter::from(haul_id).filter_str()?;

        let result = Trip::index(self)
            .search()
            .with_filter(&filter)
            .with_limit(1)
            .execute::<Trip>()
            .await
            .change_context(MeilisearchError::Query)?;

        let trip = result
            .hits
            .into_iter()
            .next()
            .map(|h| h.result.try_to_trip_detailed(read_fishing_facility))
            .transpose()?;

        Ok(trip)
    }

    pub(crate) async fn trip_of_landing_impl(
        &self,
        landing_id: &LandingId,
        read_fishing_facility: bool,
    ) -> Result<Option<TripDetailed>, MeilisearchError> {
        let filter = TripFilter::from(landing_id).filter_str()?;

        let result = Trip::index(self)
            .search()
            .with_filter(&filter)
            .with_limit(1)
            .execute::<Trip>()
            .await
            .change_context(MeilisearchError::Query)?;

        let trip = result
            .hits
            .into_iter()
            .next()
            .map(|h| h.result.try_to_trip_detailed(read_fishing_facility))
            .transpose()?;

        Ok(trip)
    }
}
