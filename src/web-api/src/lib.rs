#![deny(warnings)]
#![deny(rust_2018_idioms)]

use kyogre_core::{MatrixCacheOutbound, WebApiInboundPort, WebApiOutboundPort};
use postgres::PostgresAdapter;
use routes::v1;

pub mod error;
pub mod excel;
pub mod extractors;
pub mod guards;
pub mod response;
pub mod routes;
pub mod settings;
pub mod startup;
pub mod states;

pub trait Database: WebApiOutboundPort + WebApiInboundPort {}
pub trait Cache: MatrixCacheOutbound {}

impl Database for PostgresAdapter {}
impl Cache for duckdb_rs::Client {}
