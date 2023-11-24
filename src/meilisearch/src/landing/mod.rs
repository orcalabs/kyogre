use error_stack::{Result, ResultExt};
use kyogre_core::LandingsQuery;

use crate::{error::MeilisearchError, indexable::Indexable, query::Query, MeilisearchAdapter};

mod filter;
mod model;

pub use filter::*;
pub use model::*;

impl<T> MeilisearchAdapter<T> {
    pub(crate) async fn landings_impl(
        &self,
        query: LandingsQuery,
    ) -> Result<Vec<kyogre_core::Landing>, MeilisearchError> {
        let query = Query::<LandingFilter, Option<LandingSort>, _>::from(query);

        let sort_string = query.sort_str_opt();
        let sort = sort_string
            .as_ref()
            .map(|s| vec![s.as_str()])
            .unwrap_or_default();

        let filter = query.filter_strs()?;
        let filter = filter.iter().map(|f| f.as_str()).collect();

        let result = Landing::index(self)
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
            .map(|h| h.result.into())
            .collect::<Vec<_>>();

        Ok(landings)
    }
}
