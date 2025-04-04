use crate::{MeilisearchAdapter, error::Result, indexable::Indexable, query::Query};
use kyogre_core::{Landings, LandingsQuery, Pagination};

mod filter;
mod model;

pub use filter::*;
pub use model::*;

impl<T> MeilisearchAdapter<T> {
    pub(crate) async fn landings_impl(
        &self,
        query: LandingsQuery,
    ) -> Result<Vec<kyogre_core::Landing>> {
        let query = Query::<LandingFilter, Option<LandingSort>, Pagination<Landings>>::from(query);

        let pagination = query.pagination;

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
            .with_limit(pagination.limit() as usize)
            .with_offset(pagination.offset() as usize)
            .execute::<Landing>()
            .await?;

        let landings = result
            .hits
            .into_iter()
            .map(|h| h.result.into())
            .collect::<Vec<_>>();

        Ok(landings)
    }
}
