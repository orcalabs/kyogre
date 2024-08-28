#![deny(warnings)]
#![deny(rust_2018_idioms)]

use engine::{settings::Settings, startup::App, TracingMode};
use error_stack::{fmt::ColorMode, Report};
use orca_core::{Environment, TracingOutput};
use tracing::{event, span, Level};

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
        }
        TracingMode::TokioConsole => console_subscriber::init(),
    }

    let app = App::build(&settings).await;

    let span = span!(Level::TRACE, "engine");
    let _enter = span.enter();

    event!(Level::INFO, "starting engine...");

    app.run().await;
}
