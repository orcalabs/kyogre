use config::{Config, ConfigError, File};
use once_cell::sync::OnceCell;
use orca_core::{Environment, LogLevel, PsqlSettings, TelemetrySettings};
use serde::Deserialize;

use crate::cache::CacheErrorMode;

pub static BW_PROFILES_URL: OnceCell<String> = OnceCell::new();

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub telemetry: Option<TelemetrySettings>,
    pub api: ApiSettings,
    pub postgres: PsqlSettings,
    pub meilisearch: Option<meilisearch::Settings>,
    pub environment: Environment,
    pub honeycomb: Option<HoneycombApiKey>,
    pub bw_settings: Option<BwSettings>,
    pub duck_db_api: Option<Duckdb>,
    pub auth0: Option<Auth0Settings>,
    pub cache_error_mode: Option<CacheErrorMode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Duckdb {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BwSettings {
    pub jwks_url: String,
    pub audience: String,
    pub profiles_url: String,
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

#[derive(Debug, Clone, Deserialize)]
pub struct Auth0Settings {
    pub jwk_url: String,
    pub authorization_url: String,
    pub client_id: String,
    pub audience: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .unwrap_or(Environment::Test);

        let environment = environment.as_str().to_lowercase();

        let settings: Settings = Config::builder()
            .add_source(File::with_name(&format!("config/{}", environment)).required(true))
            .add_source(File::with_name(&format!("config/{}.secret", environment)).required(false))
            .add_source(config::Environment::with_prefix("KYOGRE_API").separator("__"))
            .set_override("environment", environment.as_str())?
            .build()?
            .try_deserialize()?;

        if let Some(ref bw) = settings.bw_settings {
            BW_PROFILES_URL.set(bw.profiles_url.clone()).unwrap();
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
