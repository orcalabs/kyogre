use config::{Config, File, Source};
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub postgres: PsqlSettings,
}

impl Settings {
    pub fn new() -> Result<Settings, config::ConfigError> {
        let environment: Environment = std::env::var("APP_ENVIRONMENT")
            .unwrap()
            .try_into()
            .expect("Failed to parse APP_ENVIRONMENT.");

        let mut builder = Config::builder().add_source(
            File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                .required(true),
        );

        if environment == Environment::Development {
            let database = config::File::with_name("/run/secrets/postgres-credentials.yaml")
                .required(true)
                .format(config::FileFormat::Yaml);
            let map = database.collect()?;
            builder = builder.set_override("postgres.ip", map["ip"].clone())?;
            builder = builder.set_override("postgres.username", map["username"].clone())?;
            builder = builder.set_override("postgres.password", map["password"].clone())?;
        }

        let config = builder.build()?;

        config.try_deserialize()
    }
}
