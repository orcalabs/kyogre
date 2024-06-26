#![deny(warnings)]
#![deny(rust_2018_idioms)]

use duckdb_rs::{settings::Settings, startup::App};
use error_stack::{fmt::ColorMode, Report};
use orca_core::{Environment, TracingOutput};
use tracing::{event, span, Level};

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let tracing = match settings.environment {
        Environment::Test | Environment::Local | Environment::Production => TracingOutput::Local,
        Environment::Development | Environment::Staging => {
            Report::<()>::set_color_mode(ColorMode::None);
            TracingOutput::Honeycomb {
                api_key: settings.honeycomb_api_key(),
            }
        }
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-duckdb",
        "duckdb",
        tracing,
    );

    let app = App::build(&settings).await;

    let span = span!(Level::TRACE, "duckdb");
    let _enter = span.enter();

    event!(Level::INFO, "starting duckdb...");

    app.run().await
}
