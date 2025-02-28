use crate::{AisVms, Cluster, ErsTripAssembler, FisheryDiscriminants, LandingTripAssembler};
use config::ConfigError;
use kyogre_core::*;
use orca_core::{Environment, PsqlSettings};
use processors::{AisVmsConflict, UnrealisticSpeed};
use serde::Deserialize;
use trip_benchmark::*;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub num_trip_state_workers: u32,
    pub local_processing_vessels: Option<Vec<FiskeridirVesselId>>,
    pub postgres: PsqlSettings,
    pub meilisearch: Option<meilisearch::Settings>,
    pub environment: Environment,
    pub scraper: scraper::Config,
    pub single_state_run: Option<FisheryDiscriminants>,
    pub fishing_predictors: Option<FishingPredictorSettings>,
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

impl Settings {
    pub fn new(settings: orca_core::Settings) -> Result<Self, ConfigError> {
        settings.config("KYOGRE_ENGINE")
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
    pub fn benchmarks(&self) -> Vec<Box<dyn TripBenchmark>> {
        // Order is significant as some benchmarks depends on the output of others.
        // Currently most benchmarks depends on 'FuelConsumption'.
        vec![
            Box::<FuelConsumption>::default(),
            Box::<WeightPerHour>::default(),
            Box::<WeightPerDistance>::default(),
            Box::<WeightPerFuel>::default(),
            Box::<CatchValuePerFuel>::default(),
            Box::<Eeoi>::default(),
            // `Sustainability` needs to be last because it depends on benchmarks above.
            // TODO
            // Box::<Sustainability>::default(),
        ]
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
