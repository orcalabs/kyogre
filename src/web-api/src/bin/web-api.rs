#![deny(warnings)]
#![deny(rust_2018_idioms)]

use error_stack::{fmt::ColorMode, Report};
use orca_core::{Environment, TracingOutput};
use tracing::Level;
use web_api::{settings::Settings, startup::App};

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

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
        "kyogre-fishery-api",
        "fishery-api",
        tracing,
    );

    let app = App::build(&settings).await;

    app.run().await.unwrap();
}
