use error_stack::{Result, ResultExt};
use kyogre_core::HaulsQuery;

use crate::{error::MeilisearchError, indexable::Indexable, MeilisearchAdapter};

mod model;
mod query;

pub use model::*;

use query::{HaulSort, Query};

impl<T> MeilisearchAdapter<T> {
    pub(crate) async fn hauls_impl(
        &self,
        query: HaulsQuery,
    ) -> Result<Vec<kyogre_core::Haul>, MeilisearchError> {
        let sort_string = query.sorting.map(|sorting| {
            format!(
                "{}:{}",
                HaulSort::from(sorting),
                query.ordering.unwrap_or_default(),
            )
        });
        let sort = sort_string
            .as_ref()
            .map(|s| vec![s.as_str()])
            .unwrap_or_default();

        let query = Query::from(query);

        let filter = query.filter_strs()?;
        let filter = filter.iter().map(|f| f.as_str()).collect();

        let result = Haul::index(self)
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
            .map(|h| h.result.into())
            .collect::<Vec<_>>();

        Ok(hauls)
    }
}
