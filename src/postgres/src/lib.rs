#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod adapter;
mod error;
mod landing_set;
mod models;
mod queries;
mod test_db;

pub use adapter::PostgresAdapter;
pub use test_db::TestDb;
