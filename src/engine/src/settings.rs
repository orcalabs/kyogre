use crate::TripProcessor;
use config::{Config, ConfigError, File, Source};
use haul_distributor::HaulDistributor;
use orca_core::{Environment, LogLevel, PsqlSettings, TelemetrySettings};
use serde::Deserialize;
use trip_distancer::TripDistancer;
use vessel_benchmark::*;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub telemetry: Option<TelemetrySettings>,
    pub postgres: PsqlSettings,
    pub engine: crate::Config,
    pub environment: Environment,
    pub scraper: scraper::Config,
    pub honeycomb: Option<HoneycombApiKey>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HoneycombApiKey {
    pub api_key: String,
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
            .add_source(config::Environment::with_prefix("KYOGRE_ENGINE").separator("__"))
            .set_override("environment", environment.as_str())?;

        if environment == Environment::Development {
            let database = config::File::with_name("/run/secrets/postgres-credentials.yaml")
                .required(true)
                .format(config::FileFormat::Yaml);
            let map = database.collect()?;
            builder = builder.set_override("postgres.ip", map["ip"].clone())?;
            builder = builder.set_override("postgres.username", map["username"].clone())?;
            builder = builder.set_override("postgres.password", map["password"].clone())?;

            let honeycomb = config::File::with_name("/run/secrets/honeycomb-api-key")
                .required(true)
                .format(config::FileFormat::Yaml);

            let map = honeycomb.collect()?;
            builder = builder.set_override("honeycomb.api_key", map["api-key"].clone())?;

            let oauth_settings =
                config::File::with_name("/run/secrets/barentswatch-api-oauth.yaml")
                    .required(true)
                    .format(config::FileFormat::Yaml);
            let map = oauth_settings.collect()?;
            builder = builder.set_override(
                "scraper.fishing_facility.client_id",
                map["client_id"].clone(),
            )?;
            builder = builder.set_override(
                "scraper.fishing_facility.client_secret",
                map["client_secret"].clone(),
            )?;
            builder = builder.set_override(
                "scraper.fishing_facility_historic.client_id",
                map["client_id"].clone(),
            )?;
            builder = builder.set_override(
                "scraper.fishing_facility_historic.client_secret",
                map["client_secret"].clone(),
            )?;
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

    pub fn trip_assemblers(&self) -> Vec<Box<dyn TripProcessor>> {
        let landings_assembler = Box::<trip_assembler::LandingTripAssembler>::default();
        let ers_assembler = Box::<trip_assembler::ErsTripAssembler>::default();

        let vec = vec![
            ers_assembler as Box<dyn TripProcessor>,
            landings_assembler as Box<dyn TripProcessor>,
        ];

        vec
    }
    pub fn benchmarks(&self) -> Vec<Box<dyn VesselBenchmark>> {
        let weight_per_hour = Box::<WeightPerHour>::default();

        let vec = vec![weight_per_hour as Box<dyn VesselBenchmark>];

        vec
    }
    pub fn haul_distributors(&self) -> Vec<Box<dyn HaulDistributor>> {
        vec![Box::<haul_distributor::AisVms>::default()]
    }
    pub fn trip_distancers(&self) -> Vec<Box<dyn TripDistancer>> {
        vec![Box::<trip_distancer::AisVms>::default()]
    }
}
