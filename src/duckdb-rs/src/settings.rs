use config::ConfigError;
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

use crate::adapter::DuckdbSettings;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub postgres: PsqlSettings,
    pub environment: Environment,
    pub duck_db: DuckdbSettings,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
}

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, ConfigError> {
        settings.config("KYOGRE_DUCKDB")
    }
}
