use crate::{MeilisearchAdapter, error::Result, indexable::Indexable, query::Query};
use kyogre_core::HaulsQuery;

mod filter;
mod model;

pub use filter::*;
pub use model::*;

impl<T> MeilisearchAdapter<T> {
    pub(crate) async fn hauls_impl(&self, query: HaulsQuery) -> Result<Vec<kyogre_core::Haul>> {
        let query = Query::<HaulFilter, Option<HaulSort>, _>::from(query);

        let sort_string = query.sort_str_opt();
        let sort = sort_string
            .as_ref()
            .map(|s| vec![s.as_str()])
            .unwrap_or_default();

        let filter = query.filter_strs()?;
        let filter = filter.iter().map(|f| f.as_str()).collect();

        let result = Haul::index(self)
            .search()
            .with_array_filter(filter)
            .with_limit(usize::MAX)
            .with_sort(&sort)
            .execute::<Haul>()
            .await?;

        let hauls = result
            .hits
            .into_iter()
            .map(|h| h.result.into())
            .collect::<Vec<_>>();

        Ok(hauls)
    }
}
