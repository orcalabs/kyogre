#![deny(warnings)]
#![deny(rust_2018_idioms)]

use orca_core::{Environment, PsqlSettings};
use postgres::PostgresAdapter;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub postgres: PsqlSettings,
    pub environment: Environment,
}

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, config::ConfigError> {
        settings.config("KYOGRE_DATABASE_MIGRATOR")
    }
}

#[tokio::main]
async fn main() {
    let settings = orca_core::Settings::new().unwrap();
    settings.init_tracer("kyogre-database-migrator", "database-migrator");

    let settings = Settings::new(settings).unwrap();

    PostgresAdapter::new(&settings.postgres)
        .await
        .unwrap()
        .do_migrations()
        .await;
}
