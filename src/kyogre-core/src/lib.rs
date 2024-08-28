#![deny(warnings)]
#![deny(rust_2018_idioms)]

pub static KEEP_DB_ENV: &str = "KEEP_TEST_DB";

use async_trait::async_trait;

mod distance_to_shore;
mod domain;
mod error;
mod oauth;
mod ports;
mod queries;

pub use distance_to_shore::*;
pub use domain::*;
pub use error::*;
pub use oauth::*;
pub use ports::*;
pub use queries::*;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn run(&self);
}
