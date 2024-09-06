#![deny(warnings)]
#![deny(rust_2018_idioms)]

use config::{Config, File};
use orca_core::{Environment, LogLevel, PsqlSettings, TracingOutput};
use postgres::PostgresAdapter;
use serde::Deserialize;
use tracing::Level;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub postgres: PsqlSettings,
    pub log_level: LogLevel,
    pub honeycomb: Option<HoneycombApiKey>,
    pub environment: Environment,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
}

impl Settings {
    pub fn prefix() -> &'static str {
        "KYOGRE_DATABASE_MIGRATOR"
    }
    pub fn honeycomb_api_key(&self) -> String {
        self.honeycomb.clone().unwrap().api_key
    }
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .unwrap_or(Environment::Test);

        Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .add_source(config::Environment::with_prefix(Self::prefix()).separator("__"))
            .set_override("environment", environment.as_str())?
            .build()?
            .try_deserialize()
    }
}

#[tokio::main]
async fn main() {
    let settings = Settings::new().unwrap();

    let tracing = match settings.environment {
        Environment::Test | Environment::Production | Environment::Local => TracingOutput::Local,
        Environment::Development => TracingOutput::Honeycomb {
            api_key: settings.honeycomb_api_key(),
        },
        Environment::OnPremise => {
            panic!("should never run database-migrator in OnPremise environment");
        }
    };
    orca_core::init_tracer(
        Level::from(&settings.log_level),
        "kyogre-database-migrator",
        "database-migrator",
        tracing,
    );

    PostgresAdapter::new(&settings.postgres)
        .await
        .unwrap()
        .do_migrations()
        .await;
}
