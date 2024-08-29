#![deny(warnings)]
#![deny(rust_2018_idioms)]

use engine::{settings::Settings, startup::App, TracingMode};
use orca_core::{Environment, TracingOutput};
use tracing::Level;

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    match settings.tracing_mode {
        TracingMode::Regular => {
            let tracing = match settings.environment {
                Environment::Test
                | Environment::Local
                | Environment::Production
                | Environment::OnPremise => TracingOutput::Local,
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
        }
        TracingMode::TokioConsole => console_subscriber::init(),
    }

    let app = App::build(&settings).await;

    app.run().await;
}
