use std::collections::HashMap;

use config::ConfigError;
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

use crate::extractors::AcceptedIssuer;

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
    pub audience: String,
    pub issuers: HashMap<AcceptedIssuer, BwEnvironmentSettings>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BwEnvironmentSettings {
    pub jwks_url: String,
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
        settings.config("KYOGRE_API")
    }
}

impl ApiSettings {
    pub fn listener_address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}
