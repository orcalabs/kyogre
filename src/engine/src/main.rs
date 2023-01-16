#![deny(warnings)]
#![deny(rust_2018_idioms)]

use engine::{settings::Settings, startup::App};
use orca_core::{Environment, TracingOutput};
use tracing::{event, span, Level};

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
        "kyogre-engine",
        "engine",
        tracing,
    );

    let app = App::build(&settings).await;

    let span = span!(Level::TRACE, "engine");
    let _enter = span.enter();

    event!(Level::INFO, "starting engine...");

    app.run().await;
}
