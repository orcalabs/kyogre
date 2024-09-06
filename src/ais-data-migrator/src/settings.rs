use chrono::{DateTime, Utc};
use config::ConfigError;
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub source: PsqlSettings,
    pub destination: PsqlSettings,
    #[serde(with = "humantime_serde")]
    pub chunk_size: std::time::Duration,
    pub start_threshold: DateTime<Utc>,
    pub end_threshold: DateTime<Utc>,
    pub environment: Environment,
}

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, ConfigError> {
        settings.config("KYOGRE_AIS_DATA_MIGRATOR")
    }
}
