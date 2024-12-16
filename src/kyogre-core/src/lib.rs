#![deny(warnings)]
#![deny(rust_2018_idioms)]

//! The main crate defining all shared traits and models.

pub static POSTGRES_TEST_PORT: u32 = 5534;
pub static POSTGRES_TEST_MASTER_PORT: u32 = 5535;

use async_trait::async_trait;

mod distance_to_shore;
mod domain;
mod error;
mod mean;
mod oauth;
mod ports;
mod queries;
mod retry;

pub use distance_to_shore::*;
pub use domain::*;
pub use error::*;
pub use mean::*;
pub use oauth::*;
pub use ports::*;
pub use queries::*;
pub use retry::*;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn run(&self);
}

pub trait EmptyVecToNone
where
    Self: Sized,
{
    fn empty_to_none(self) -> Option<Self>;
}

impl<T> EmptyVecToNone for Vec<T> {
    fn empty_to_none(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }
}

impl<T> EmptyVecToNone for &[T] {
    fn empty_to_none(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }
}
