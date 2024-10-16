#![deny(warnings)]
#![deny(rust_2018_idioms)]

pub static POSTGRES_TEST_PORT: u32 = 5534;

use async_trait::async_trait;

mod distance_to_shore;
mod domain;
mod error;
mod oauth;
mod ports;
mod queries;
mod retry;

pub use distance_to_shore::*;
pub use domain::*;
pub use error::*;
pub use oauth::*;
pub use ports::*;
pub use queries::*;
pub use retry::*;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn run(&self);
}
