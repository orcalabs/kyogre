#![deny(warnings)]
#![deny(rust_2018_idioms)]

use engine::{settings::Settings, startup::App};
use error_stack::{fmt::ColorMode, Report};
use orca_core::{Environment, TracingOutput};
use tokio::select;
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
                api_key: settings.honeycomb_api_key(),
            }
        }
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-engine",
        "engine",
        tracing,
    );

    let (engine, meilisearch) = App::build(&settings).await;

    let span = span!(Level::TRACE, "engine");
    let _enter = span.enter();

    event!(Level::INFO, "starting engine...");

    if let Some(meilisearch) = meilisearch {
        let engine = tokio::spawn(engine.run());
        let meilisearch = tokio::spawn(meilisearch.run());

        select! {
            _ = engine => {
                event!(Level::ERROR, "engine exited unexpectedly");
            },
            _ = meilisearch => {
                event!(Level::ERROR, "meilisearch exited unexpectedly");
            },
        }
    } else {
        engine.run().await;
    }
}
