#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod adapter;
mod chunk;
mod error;
mod ers_dca_set;
mod ers_dep_set;
mod ers_por_set;
mod ers_tra_set;
mod landing_set;
mod models;
mod queries;
#[cfg(feature = "test")]
mod test_db;

pub use adapter::PostgresAdapter;
#[cfg(feature = "test")]
pub use test_db::TestDb;
