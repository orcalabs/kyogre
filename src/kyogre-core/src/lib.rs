#![deny(warnings)]
#![deny(rust_2018_idioms)]

pub static KEEP_DB_ENV: &str = "KEEP_TEST_DB";

use async_trait::async_trait;

mod domain;
mod error;
mod file_hash;
mod oauth;
mod ports;
mod queries;

pub use domain::*;
pub use error::*;
pub use file_hash::*;
pub use oauth::*;
pub use ports::*;
pub use queries::*;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn run(&self);
}
