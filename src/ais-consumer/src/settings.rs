use config::{Config, File, Source};
use orca_core::{Environment, LogLevel, PsqlSettings};
use serde::Deserialize;

use crate::token::OauthConfig;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub log_level: LogLevel,
    pub postgres: PsqlSettings,
    pub environment: Environment,
    #[serde(with = "humantime_serde")]
    pub commit_interval: std::time::Duration,
    pub broadcast_buffer_size: usize,
    pub oauth: Option<OauthConfig>,
    pub api_address: Option<String>,
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
            .add_source(config::Environment::with_prefix("AIS_CONSUMER").separator("__"))
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

            let oauth_settings =
                config::File::with_name("/run/secrets/barentswatch-ais-oauth.yaml")
                    .required(true)
                    .format(config::FileFormat::Yaml);
            let map = oauth_settings.collect()?;
            builder = builder.set_override("client_secret", map["client_secret"].clone())?;
            builder = builder.set_override("client_id", map["client_id"].clone())?;
        }

        let config = builder.build()?;

        config.try_deserialize()
    }
}
