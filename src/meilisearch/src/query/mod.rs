use crate::error::Result;
use kyogre_core::Ordering;
use std::{collections::BTreeSet, fmt::Display};

mod haul;
mod landing;
mod trip;

pub struct Query<F, S, P> {
    filters: BTreeSet<F>,
    sorting: S,
    ordering: Ordering,
    pub pagination: P,
}

impl<F, S, P> Query<F, S, P>
where
    F: Filter,
{
    pub fn filter_strs(self) -> Result<Vec<String>> {
        self.filters
            .into_iter()
            .map(|f| f.filter_str())
            .collect::<Result<_>>()
    }
}

impl<F, S, P> Query<F, S, P>
where
    S: Display + Copy,
{
    pub fn sort_str(&self) -> String {
        sort_str(self.sorting, self.ordering)
    }
}

impl<F, S, P> Query<F, Option<S>, P>
where
    S: Display + Copy,
{
    pub fn sort_str_opt(&self) -> Option<String> {
        self.sorting.map(|s| sort_str(s, self.ordering))
    }
}

pub trait Filter {
    fn filter_str(self) -> Result<String>;
}

fn sort_str<S: Display>(sorting: S, ordering: Ordering) -> String {
    format!("{sorting}:{ordering}")
}
