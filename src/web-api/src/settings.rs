use std::sync::OnceLock;

use config::ConfigError;
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

pub static BW_PROFILES_URL: OnceLock<String> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub api: ApiSettings,
    pub postgres: PsqlSettings,
    pub environment: Environment,
    pub bw_settings: Option<BwSettings>,
    pub duck_db_api: Option<Duckdb>,
    pub auth0: Option<Auth0Settings>,
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
    pub audience: String,
}

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, ConfigError> {
        let settings: Settings = settings.config("KYOGRE_API")?;

        if let Some(ref bw) = settings.bw_settings {
            BW_PROFILES_URL.set(bw.profiles_url.clone()).unwrap();
        }

        Ok(settings)
    }
}

impl ApiSettings {
    pub fn listener_address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}
