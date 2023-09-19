use crate::*;
use machine::{Machine, Schedule};
use serde::Deserialize;

pub mod states;

pub use states::*;
use strum::EnumDiscriminants;

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
    Trips(TripsState),
    Benchmark(BenchmarkState),
    HaulDistribution(HaulDistributionState),
    HaulWeather(HaulWeatherState),
    VerifyDatabase(VerifyDatabaseState),
}

// TODO: change do Box<dyn Database> after (https://github.com/rust-lang/rust/issues/65991) resolves.
pub struct SharedState {
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
    pub haul_distributors: Vec<Box<dyn HaulDistributor>>,
    pub trip_distancer: Box<dyn TripDistancer>,
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
        scraper: Option<Box<dyn Scraper>>,
        trip_assemblers: Vec<Box<dyn TripAssembler>>,
        benchmarks: Vec<Box<dyn VesselBenchmark>>,
        haul_distributors: Vec<Box<dyn HaulDistributor>>,
        trip_distancer: Box<dyn TripDistancer>,
    ) -> SharedState {
        SharedState {
            scraper,
            trip_assemblers,
            benchmarks,
            haul_distributors,
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
        }
    }
}
