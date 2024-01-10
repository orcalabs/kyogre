use crate::{
    AisVms, AisVmsConflict, Cluster, ErsTripAssembler, FisheryDiscriminants, LandingTripAssembler,
    UnrealisticSpeed,
};
use config::{Config, ConfigError, File};
use kyogre_core::*;
use orca_core::{Environment, LogLevel, PsqlSettings, TelemetrySettings};
use serde::Deserialize;
use vessel_benchmark::*;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub log_level: LogLevel,
    pub num_workers: u32,
    pub tracing_mode: TracingMode,
    pub telemetry: Option<TelemetrySettings>,
    pub postgres: PsqlSettings,
    pub meilisearch: Option<meilisearch::Settings>,
    pub environment: Environment,
    pub scraper: scraper::Config,
    pub honeycomb: Option<HoneycombApiKey>,
    pub single_state_run: Option<FisheryDiscriminants>,
    pub fishing_predictors: Option<FishingPredictorSettings>,
}

#[derive(Debug, Deserialize)]
pub enum TracingMode {
    Regular,
    TokioConsole,
}

#[derive(Debug, Deserialize)]
pub struct FishingPredictorSettings {
    pub training_rounds: u32,
    pub training_mode: TrainingMode,
}

#[derive(Debug, Deserialize)]
pub struct MatrixClientSettings {
    pub ip: String,
    pub port: u16,
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

        Config::builder()
            .add_source(
                File::with_name(&format!("config/{}", environment.as_str().to_lowercase()))
                    .required(true),
            )
            .add_source(config::Environment::with_prefix("KYOGRE_ENGINE").separator("__"))
            .set_override("environment", environment.as_str())?
            .build()?
            .try_deserialize()
    }

    pub fn telemetry_endpoint(&self) -> Option<String> {
        self.telemetry.as_ref().map(|t| t.endpoint())
    }

    pub fn honeycomb_api_key(&self) -> String {
        self.honeycomb.clone().unwrap().api_key
    }

    pub fn trip_assemblers(&self) -> Vec<Box<dyn TripAssembler>> {
        let landings_assembler = Box::<LandingTripAssembler>::default();
        let ers_assembler = Box::<ErsTripAssembler>::default();

        let vec = vec![
            ers_assembler as Box<dyn TripAssembler>,
            landings_assembler as Box<dyn TripAssembler>,
        ];

        vec
    }
    pub fn ml_models(&self) -> Vec<Box<dyn MLModel>> {
        vec![]
    }
    pub fn benchmarks(&self) -> Vec<Box<dyn VesselBenchmark>> {
        let weight_per_hour = Box::<WeightPerHour>::default();

        let vec = vec![weight_per_hour as Box<dyn VesselBenchmark>];

        vec
    }
    pub fn trip_distancer(&self) -> Box<dyn TripDistancer> {
        Box::<AisVms>::default()
    }

    pub fn trip_position_layers(&self) -> Vec<Box<dyn TripPositionLayer>> {
        vec![
            Box::<AisVmsConflict>::default(),
            Box::<UnrealisticSpeed>::default(),
            Box::<Cluster>::default(),
        ]
    }
}
