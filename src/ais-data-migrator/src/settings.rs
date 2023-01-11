use chrono::{DateTime, Utc};
use config::{Config, File, Source};
use orca_core::{Environment, LogLevel, PsqlSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub source: PsqlSettings,
    pub destination: PsqlSettings,
    #[serde(with = "humantime_serde")]
    pub chunk_size: std::time::Duration,
    pub source_start_threshold: DateTime<Utc>,
    pub destination_end_threshold: DateTime<Utc>,
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
            .expect("Failed to parse APP_ENVIRONMENT.");

        let mut builder = Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .set_override("environment", environment.as_str())?;

        if environment == Environment::Development {
            let database = config::File::with_name("/run/secrets/postgres-credentials.yaml")
                .required(true)
                .format(config::FileFormat::Yaml);
            let map = database.collect()?;
            builder = builder.set_override("postgres.ip", map["ip"].clone())?;
            builder = builder.set_override("postgres.username", map["username"].clone())?;
            builder = builder.set_override("postgres.password", map["password"].clone())?;

            let honeycomb = config::File::with_name("/run/secrets/honeycomb-api-key")
                .required(true)
                .format(config::FileFormat::Yaml);
            let map = honeycomb.collect()?;
            builder = builder.set_override("honeycomb.api_key", map["api-key"].clone())?;
        }

        let config = builder.build()?;

        config.try_deserialize()
    }
}
