use consumer::{settings::Settings, startup::App};
use orca_core::{Environment, TracingOutput};
use tracing::{event, span, Level};

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let tracing = match settings.environment {
        Environment::Development
        | Environment::Test
        | Environment::Local
        | Environment::Production
        | Environment::Staging => TracingOutput::Local,
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "ais-ais-consumer",
        "ais-consumer",
        tracing,
    );

    let _app = App::build(settings, None).await;

    let span = span!(Level::TRACE, "ais_consumer");
    let _enter = span.enter();

    event!(Level::INFO, "starting ais_consumer...");

    // app.run().await;
}
