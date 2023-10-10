#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod adapter;
mod error;
mod ers_dca_set;
mod ers_dep_set;
mod ers_por_set;
mod ers_tra_set;
mod landing_set;
mod models;
mod queries;
mod test_db;
mod fft;

pub use adapter::PostgresAdapter;
pub use test_db::TestDb;
