use crate::*;
use machine::{Machine, Schedule};

pub mod states;

pub use states::*;

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

#[derive(Machine)]
#[machine(shared_state = SharedState, order_chain)]
pub enum Fishery {
    Scrape(ScrapeState),
    Trips(TripsState),
    TripsPrecision(TripsPrecisionState),
    Benchmark(BenchmarkState),
    HaulDistribution(HaulDistributionState),
    TripDistance(TripsDistanceState),
    UpdateDatabaseViews(UpdateDatabaseViewsState),
    VerifyDatabase(VerifyDatabaseState),
}

// TODO: change do Box<dyn Database> after (https://github.com/rust-lang/rust/issues/65991) resolves.
pub struct SharedState {
    pub trip_assembler_outbound_port: Box<dyn TripAssemblerOutboundPort>,
    pub trips_precision_outbound_port: Box<dyn TripPrecisionOutboundPort>,
    pub trips_precision_inbound_port: Box<dyn TripPrecisionInboundPort>,
    pub refresher: Box<dyn DatabaseViewRefresher>,
    pub verifier: Box<dyn VerificationOutbound>,
    pub matrix_cache: Box<dyn MatrixCacheVersion>,
    pub benchmark_inbound: Box<dyn VesselBenchmarkInbound>,
    pub benchmark_outbound: Box<dyn VesselBenchmarkOutbound>,
    pub haul_distributor_inbound: Box<dyn HaulDistributorInbound>,
    pub haul_distributor_outbound: Box<dyn HaulDistributorOutbound>,
    pub trip_distancer_inbound: Box<dyn TripDistancerInbound>,
    pub trip_distancer_outbound: Box<dyn TripDistancerOutbound>,
    pub scraper: Option<Box<dyn Scraper>>,
    pub trip_assemblers: Vec<Box<dyn TripAssembler>>,
    pub benchmarks: Vec<Box<dyn VesselBenchmark>>,
    pub haul_distributors: Vec<Box<dyn HaulDistributor>>,
    pub trip_distancers: Vec<Box<dyn TripDistancer>>,
}

impl SharedState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trip_assembler_outbound_port: Box<dyn TripAssemblerOutboundPort>,
        trips_precision_outbound_port: Box<dyn TripPrecisionOutboundPort>,
        trips_precision_inbound_port: Box<dyn TripPrecisionInboundPort>,
        verifier: Box<dyn VerificationOutbound>,
        refresher: Box<dyn DatabaseViewRefresher>,
        matrix_cache: Box<dyn MatrixCacheVersion>,
        benchmark_inbound: Box<dyn VesselBenchmarkInbound>,
        benchmark_outbound: Box<dyn VesselBenchmarkOutbound>,
        haul_distributor_inbound: Box<dyn HaulDistributorInbound>,
        haul_distributor_outbound: Box<dyn HaulDistributorOutbound>,
        trip_distancer_inbound: Box<dyn TripDistancerInbound>,
        trip_distancer_outbound: Box<dyn TripDistancerOutbound>,
        scraper: Option<Box<dyn Scraper>>,
        trip_assemblers: Vec<Box<dyn TripAssembler>>,
        benchmarks: Vec<Box<dyn VesselBenchmark>>,
        haul_distributors: Vec<Box<dyn HaulDistributor>>,
        trip_distancers: Vec<Box<dyn TripDistancer>>,
    ) -> SharedState {
        SharedState {
            scraper,
            trip_assemblers,
            benchmarks,
            haul_distributors,
            trip_distancers,
            trip_assembler_outbound_port,
            trips_precision_outbound_port,
            trips_precision_inbound_port,
            refresher,
            verifier,
            matrix_cache,
            benchmark_inbound,
            benchmark_outbound,
            haul_distributor_inbound,
            haul_distributor_outbound,
            trip_distancer_inbound,
            trip_distancer_outbound,
        }
    }
}
