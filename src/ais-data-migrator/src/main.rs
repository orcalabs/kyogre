#![deny(warnings)]
#![deny(rust_2018_idioms)]

use ais_data_migrator::{settings::Settings, startup::App};
use orca_core::{Environment, TracingOutput};
use tracing::Level;

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let tracing = match settings.environment {
        Environment::Test
        | Environment::Local
        | Environment::Production
        | Environment::OnPremise => TracingOutput::Local,
        Environment::Development => TracingOutput::Honeycomb {
            api_key: settings.honeycomb.clone().unwrap().api_key,
        },
    };

    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-ais-data-migrator",
        "ais-data-migrator",
        tracing,
    );

    let app = App::build(&settings).await;

    app.run().await;
}
