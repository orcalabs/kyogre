#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;

mod domain;
mod engine;
mod error;
mod file_hash;
mod oauth;
mod ports;
mod queries;
mod test_helper;

pub use domain::*;
pub use engine::*;
pub use error::*;
pub use file_hash::*;
pub use oauth::*;
pub use ports::*;
pub use queries::*;
pub use test_helper::*;

#[async_trait]
pub trait Scraper: Send + Sync {
    async fn run(&self);
}
