#![deny(warnings)]
#![deny(rust_2018_idioms)]

use ais_data_migrator::{settings::Settings, startup::App};
use error_stack::{fmt::ColorMode, Report};
use orca_core::{Environment, TracingOutput};
use tracing::{event, span, Level};

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let tracing = match settings.environment {
        Environment::Test | Environment::Local | Environment::Production | Environment::Staging => {
            TracingOutput::Local
        }
        Environment::Development => {
            Report::<()>::set_color_mode(ColorMode::None);
            TracingOutput::Honeycomb {
                api_key: settings.honeycomb.clone().unwrap().api_key,
            }
        }
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-ais-data-migrator",
        "ais-data-migrator",
        tracing,
    );

    let app = App::build(&settings).await;

    let span = span!(Level::TRACE, "ais_data_migrator");
    let _enter = span.enter();

    event!(Level::INFO, "starting ais_data_migrator...");

    app.run().await;
}
