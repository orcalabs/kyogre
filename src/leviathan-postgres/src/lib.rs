#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod adapter;
mod error;
mod models;

pub use adapter::LeviathanPostgresAdapter;
