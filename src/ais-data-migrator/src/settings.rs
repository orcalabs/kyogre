use chrono::{DateTime, Utc};
use config::{Config, File};
use orca_core::{Environment, LogLevel, PsqlSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub source: PsqlSettings,
    pub destination: PsqlSettings,
    #[serde(with = "humantime_serde")]
    pub chunk_size: std::time::Duration,
    pub start_threshold: DateTime<Utc>,
    pub end_threshold: DateTime<Utc>,
    pub environment: Environment,
    pub honeycomb: Option<HoneycombApiKey>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
}

impl Settings {
    pub fn new() -> Result<Settings, config::ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .unwrap_or(Environment::Test);

        Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .add_source(
                config::Environment::with_prefix("KYOGRE_AIS_DATA_MIGRATOR").separator("__"),
            )
            .set_override("environment", environment.as_str())?
            .build()?
            .try_deserialize()
    }
}
