use ais_consumer::{settings::Settings, startup::App};
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
            api_key: settings.honeycomb.clone().unwrap().api_key,
        },
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-ais-consumer",
        "ais-consumer",
        tracing,
    );

    let app = App::build(settings).await;

    let span = span!(Level::TRACE, "ais_consumer");
    let _enter = span.enter();

    event!(Level::INFO, "starting ais_consumer...");

    app.run().await;
}
