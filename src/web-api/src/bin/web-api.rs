#![deny(warnings)]
#![deny(rust_2018_idioms)]

use orca_core::{Environment, TracingOutput};
use tracing::{event, span, Level};
use web_api::{settings::Settings, startup::App};

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let tracing = match settings.environment {
        Environment::Test | Environment::Local | Environment::Production | Environment::Staging => {
            TracingOutput::Local
        }
        Environment::Development => TracingOutput::Honeycomb {
            api_key: settings.honeycomb_api_key(),
        },
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-fishery-api",
        "fishery-api",
        tracing,
    );

    let app = App::build(&settings).await;

    let span = span!(Level::TRACE, "ais_consumer");
    let _enter = span.enter();

    event!(Level::INFO, "starting ais_consumer...");

    app.run().await.unwrap();
}
