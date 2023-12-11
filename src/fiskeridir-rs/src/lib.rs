#![deny(warnings)]
#![deny(rust_2018_idioms)]

//! Implements a library for downloading and reading data sources from Fiskeridir

mod api_downloader;
mod deserialize_utils;
mod error;
mod file_downloader;
mod models;
mod string_new_types;
mod utils;

pub use api_downloader::*;
pub use error::Error;
pub use file_downloader::*;
pub use models::*;
