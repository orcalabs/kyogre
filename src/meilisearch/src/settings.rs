use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub host: String,
    pub api_key: String,
    #[serde(with = "humantime_serde")]
    pub refresh_timeout: Option<Duration>,
    pub index_suffix: Option<String>,
}
