#![deny(warnings)]
#![deny(rust_2018_idioms)]

use kyogre_core::*;
use machine::{Machine, Schedule};
use serde::Deserialize;
use strum::EnumDiscriminants;

mod error;
mod ml_models;
mod trip_assembler;
mod trip_distancer;
mod trip_layers;

pub mod settings;
pub mod startup;
pub mod states;
pub mod test_helper;

pub use ml_models::*;
pub use settings::*;
pub use startup::*;
pub use states::*;
pub use test_helper::*;
pub use trip_assembler::*;
pub use trip_distancer::*;
pub use trip_layers::*;

#[derive(Default)]
pub struct AisVms {}

// pub trait Database:
//     TripAssemblerOutboundPort
//     + TripPrecisionInboundPort
//     + TripPrecisionOutboundPort
//     + ScraperInboundPort
//     + VesselBenchmarkOutbound
//     + VesselBenchmarkInbound
//     + HaulDistributorOutbound
//     + HaulDistributorInbound
//     + TripDistancerOutbound
//     + TripDistancerInbound
//     + MatrixCacheVersion
//     + DatabaseViewRefresher
//     + VerificationOutbound
//     + Send
//     + Sync
//     + 'static
// {
// }
// impl<T> Database for T where
//     T: TripAssemblerOutboundPort
//         + TripPrecisionInboundPort
//         + TripPrecisionOutboundPort
//         + ScraperInboundPort
//         + VesselBenchmarkOutbound
//         + VesselBenchmarkInbound
//         + HaulDistributorOutbound
//         + HaulDistributorInbound
//         + TripDistancerOutbound
//         + TripDistancerInbound
//         + MatrixCacheVersion
//         + DatabaseViewRefresher
//         + VerificationOutbound
//         + 'static
// {
// }

#[derive(Machine, EnumDiscriminants)]
#[strum_discriminants(derive(Deserialize))]
#[machine(shared_state = SharedState, order_chain)]
pub enum Fishery {
    Scrape(ScrapeState),
    CatchLocationWeather(CatchLocationWeatherState),
    Trips(TripsState),
    Benchmark(BenchmarkState),
    HaulDistribution(HaulDistributionState),
    HaulWeather(HaulWeatherState),
    MLModels(MLModelsState),
    VerifyDatabase(VerifyDatabaseState),
}

// TODO: change do Box<dyn Database> after (https://github.com/rust-lang/rust/issues/65991) resolves.
pub struct SharedState {
    pub num_workers: u32,
    pub ml_models_inbound: Box<dyn MLModelsInbound>,
    pub ml_models_outbound: Box<dyn MLModelsOutbound>,
    pub trip_assembler_outbound_port: Box<dyn TripAssemblerOutboundPort>,
    pub trips_precision_outbound_port: Box<dyn TripPrecisionOutboundPort>,
    pub trip_pipeline_inbound: Box<dyn TripPipelineInbound>,
    pub trip_pipeline_outbound: Box<dyn TripPipelineOutbound>,
    pub verifier: Box<dyn VerificationOutbound>,
    pub matrix_cache: Box<dyn MatrixCacheVersion>,
    pub benchmark_inbound: Box<dyn VesselBenchmarkInbound>,
    pub benchmark_outbound: Box<dyn VesselBenchmarkOutbound>,
    pub haul_distributor_inbound: Box<dyn HaulDistributorInbound>,
    pub haul_distributor_outbound: Box<dyn HaulDistributorOutbound>,
    pub haul_weather_inbound: Box<dyn HaulWeatherInbound>,
    pub haul_weather_outbound: Box<dyn HaulWeatherOutbound>,
    pub scraper: Option<Box<dyn Scraper>>,
    pub trip_assemblers: Vec<Box<dyn TripAssembler>>,
    pub benchmarks: Vec<Box<dyn VesselBenchmark>>,
    pub trip_distancer: Box<dyn TripDistancer>,
    pub ml_models: Vec<Box<dyn MLModel>>,
    pub trip_position_layers: Vec<Box<dyn TripPositionLayer>>,
    pub catch_location_weather: Box<dyn CatchLocationWeatherInbound>,
}

impl FisheryEngine {
    pub fn add_ml_models(&mut self, models: Vec<Box<dyn MLModel>>) {
        let shared = match self {
            FisheryEngine::Pending(s) => &mut s.shared_state,
            FisheryEngine::Sleep(s) => &mut s.shared_state,
            FisheryEngine::Scrape(s) => &mut s.shared_state,
            FisheryEngine::Trips(s) => &mut s.shared_state,
            FisheryEngine::Benchmark(s) => &mut s.shared_state,
            FisheryEngine::HaulDistribution(s) => &mut s.shared_state,
            FisheryEngine::HaulWeather(s) => &mut s.shared_state,
            FisheryEngine::VerifyDatabase(s) => &mut s.shared_state,
            FisheryEngine::MLModels(s) => &mut s.shared_state,
            FisheryEngine::CatchLocationWeather(s) => &mut s.shared_state,
        };

        shared.ml_models = models;
    }
}

impl SharedState {
    pub fn assembler_id_to_impl(&self, id: TripAssemblerId) -> &dyn TripAssembler {
        let ers_idx = self
            .trip_assemblers
            .iter()
            .position(|v| v.assembler_id() == id);
        let landings_idx = self
            .trip_assemblers
            .iter()
            .position(|v| v.assembler_id() == id);
        match id {
            TripAssemblerId::Landings => self.trip_assemblers[ers_idx.unwrap()].as_ref(),
            TripAssemblerId::Ers => self.trip_assemblers[landings_idx.unwrap()].as_ref(),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        num_workers: u32,
        ml_models_inbound: Box<dyn MLModelsInbound>,
        ml_models_outbound: Box<dyn MLModelsOutbound>,
        trip_assembler_outbound_port: Box<dyn TripAssemblerOutboundPort>,
        trips_precision_outbound_port: Box<dyn TripPrecisionOutboundPort>,
        trip_pipeline_inbound: Box<dyn TripPipelineInbound>,
        trip_pipeline_outbound: Box<dyn TripPipelineOutbound>,
        verifier: Box<dyn VerificationOutbound>,
        matrix_cache: Box<dyn MatrixCacheVersion>,
        benchmark_inbound: Box<dyn VesselBenchmarkInbound>,
        benchmark_outbound: Box<dyn VesselBenchmarkOutbound>,
        haul_distributor_inbound: Box<dyn HaulDistributorInbound>,
        haul_distributor_outbound: Box<dyn HaulDistributorOutbound>,
        haul_weather_inbound: Box<dyn HaulWeatherInbound>,
        haul_weather_outbound: Box<dyn HaulWeatherOutbound>,
        catch_location_weather: Box<dyn CatchLocationWeatherInbound>,
        scraper: Option<Box<dyn Scraper>>,
        trip_assemblers: Vec<Box<dyn TripAssembler>>,
        benchmarks: Vec<Box<dyn VesselBenchmark>>,
        trip_distancer: Box<dyn TripDistancer>,
        ml_models: Vec<Box<dyn MLModel>>,
        trip_position_layers: Vec<Box<dyn TripPositionLayer>>,
    ) -> SharedState {
        SharedState {
            num_workers,
            scraper,
            trip_assemblers,
            benchmarks,
            trip_distancer,
            trip_assembler_outbound_port,
            trips_precision_outbound_port,
            verifier,
            matrix_cache,
            benchmark_inbound,
            benchmark_outbound,
            haul_distributor_inbound,
            haul_distributor_outbound,
            haul_weather_inbound,
            haul_weather_outbound,
            trip_pipeline_inbound,
            trip_pipeline_outbound,
            ml_models,
            ml_models_inbound,
            ml_models_outbound,
            trip_position_layers,
            catch_location_weather,
        }
    }
}
