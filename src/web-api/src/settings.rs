use config::{Config, ConfigError, File, Source};
use once_cell::sync::OnceCell;
use orca_core::{Environment, LogLevel, PsqlSettings, TelemetrySettings};
use serde::Deserialize;

use crate::duckdb::DuckdbSettings;

pub static BW_PROFILES_URL: OnceCell<String> = OnceCell::new();

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub telemetry: Option<TelemetrySettings>,
    pub api: ApiSettings,
    pub postgres: PsqlSettings,
    pub environment: Environment,
    pub honeycomb: Option<HoneycombApiKey>,
    pub bw_jwks_url: Option<String>,
    pub bw_profiles_url: Option<String>,
    pub duck_db: Option<DuckdbSettings>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiSettings {
    pub ip: String,
    pub port: u16,
    pub num_workers: Option<u32>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .expect("failed to parse APP_ENVIRONMENT");

        let mut builder = Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .add_source(config::Environment::with_prefix("KYOGRE_API").separator("__"))
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

        let settings: Settings = config.try_deserialize()?;

        if let Some(ref url) = settings.bw_profiles_url {
            BW_PROFILES_URL.set(url.clone()).unwrap();
        }

        Ok(settings)
    }

    pub fn telemetry_endpoint(&self) -> Option<String> {
        self.telemetry.as_ref().map(|t| t.endpoint())
    }

    pub fn honeycomb_api_key(&self) -> String {
        self.honeycomb.clone().unwrap().api_key
    }
}

impl ApiSettings {
    pub fn listener_address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}
