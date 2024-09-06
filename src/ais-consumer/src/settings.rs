use config::ConfigError;
use kyogre_core::OauthConfig;
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub postgres: PsqlSettings,
    pub environment: Environment,
    #[serde(with = "humantime_serde")]
    pub commit_interval: std::time::Duration,
    pub broadcast_buffer_size: usize,
    pub oauth: Option<OauthConfig>,
    pub api_address: Option<String>,
}

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, ConfigError> {
        settings.config("KYOGRE_AIS_CONSUMER")
    }
}
