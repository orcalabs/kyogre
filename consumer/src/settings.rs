use config::{Config, File};
use orca_core::{Environment, LogLevel, PsqlSettings};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub log_level: LogLevel,
    pub postgres: PsqlSettings,
    pub environment: Environment,
}

impl Settings {
    pub fn new() -> Result<Settings, config::ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");

        let mut builder = Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .add_source(config::Environment::with_prefix("AIS_CONSUMER").separator("__"))
            .set_override("environment", environment.as_str())?;

        let config = builder.build()?;

        config.try_deserialize()
    }
}
