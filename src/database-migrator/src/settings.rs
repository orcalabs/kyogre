use config::{Config, File};
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub postgres: PsqlSettings,
}

impl Settings {
    pub fn new() -> Result<Settings, config::ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");

        let builder = Config::builder().add_source(
            File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                .required(true),
        );

        let config = builder.build()?;

        config.try_deserialize()
    }
}
