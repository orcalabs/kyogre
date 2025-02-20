use config::ConfigError;
use kyogre_core::FiskeridirVesselId;
use orca_core::{Environment, PsqlSettings};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub fuel_estimation_vessels: Option<Vec<FiskeridirVesselId>>,
    pub num_fuel_estimation_workers: u32,
    pub current_positions_batch_size: u32,
    pub postgres: PsqlSettings,
    pub environment: Environment,
}

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, ConfigError> {
        settings.config("KYOGRE_PROCESSORS")
    }
}
