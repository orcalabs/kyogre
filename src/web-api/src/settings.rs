use config::{Config, ConfigError, File};
use orca_core::{Environment, LogLevel, PsqlSettings, TelemetrySettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub telemetry: Option<TelemetrySettings>,
    pub api: ApiSettings,
    pub postgres: PsqlSettings,
    pub environment: Environment,
    pub honeycomb: Option<HoneycombApiKey>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiSettings {
    pub ip: String,
    pub port: u16,
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
            let honeycomb = config::File::with_name("/run/secrets/honeycomb_api_key")
                .required(true)
                .format(config::FileFormat::Yaml);

            builder = builder.add_source(honeycomb);
        }

        let config = builder.build()?;

        config.try_deserialize()
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
