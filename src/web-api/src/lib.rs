#![deny(warnings)]
#![deny(rust_2018_idioms)]

use kyogre_core::WebApiPort;
use postgres::PostgresAdapter;
use routes::v1;
use utoipa::OpenApi;

pub mod error;
pub mod response;
pub mod routes;
pub mod settings;
pub mod startup;

pub trait Database: WebApiPort {}

impl Database for PostgresAdapter {}

#[derive(OpenApi)]
#[openapi(
    paths(
        v1::ais::ais_track,
    ),
    components(
        schemas(
            error::ErrorResponse,
            error::ApiError,
            v1::ais::MinimalAisPosition,
            v1::ais::NavigationStatus,
        )
    ),
    tags(
        (name = "kyogre-api", description = "kyogre api")
    ),
)]
struct ApiDoc;
