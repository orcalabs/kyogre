#![deny(warnings)]
#![deny(rust_2018_idioms)]

use haul_distributor::*;
use kyogre_core::*;
use scraper::Scraper;
use trip_assembler::TripAssembler;
use trip_distancer::*;
use vessel_benchmark::*;

use machine::{Machine, Schedule};

pub mod error;
pub mod settings;
pub mod startup;
pub mod states;

pub use error::*;
pub use settings::*;
pub use startup::*;
pub use states::*;

pub trait Database:
    TripAssemblerOutboundPort
    + TripPrecisionInboundPort
    + TripPrecisionOutboundPort
    + ScraperInboundPort
    + VesselBenchmarkOutbound
    + VesselBenchmarkInbound
    + HaulDistributorOutbound
    + HaulDistributorInbound
    + TripDistancerOutbound
    + TripDistancerInbound
    + MatrixCacheVersion
    + DatabaseViewRefresher
    + VerificationOutbound
    + Send
    + Sync
    + 'static
{
}
pub trait TripProcessor: TripAssembler + Send + Sync + 'static {}

impl<T> Database for T where
    T: TripAssemblerOutboundPort
        + TripPrecisionInboundPort
        + TripPrecisionOutboundPort
        + ScraperInboundPort
        + VesselBenchmarkOutbound
        + VesselBenchmarkInbound
        + HaulDistributorOutbound
        + HaulDistributorInbound
        + TripDistancerOutbound
        + TripDistancerInbound
        + MatrixCacheVersion
        + DatabaseViewRefresher
        + VerificationOutbound
        + 'static
{
}
impl<T> TripProcessor for T where T: TripAssembler + 'static {}

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

pub struct SharedState {
    // TODO: change do Box<dyn Database> after (https://github.com/rust-lang/rust/issues/65991) resolves.
    pub database: postgres::PostgresAdapter,
    pub scraper: Scraper,
    pub trip_processors: Vec<Box<dyn TripProcessor>>,
    pub benchmarks: Vec<Box<dyn VesselBenchmark>>,
    pub haul_distributors: Vec<Box<dyn HaulDistributor>>,
    pub trip_distancers: Vec<Box<dyn TripDistancer>>,
}

impl SharedState {
    pub fn new(
        database: postgres::PostgresAdapter,
        scraper: Scraper,
        trip_processors: Vec<Box<dyn TripProcessor>>,
        benchmarks: Vec<Box<dyn VesselBenchmark>>,
        haul_distributors: Vec<Box<dyn HaulDistributor>>,
        trip_distancers: Vec<Box<dyn TripDistancer>>,
    ) -> SharedState {
        SharedState {
            database,
            scraper,
            trip_processors,
            benchmarks,
            haul_distributors,
            trip_distancers,
        }
    }

    // TODO: remove workaround after (https://github.com/rust-lang/rust/issues/65991) resolves.
    pub fn postgres_adapter(&self) -> &postgres::PostgresAdapter {
        &self.database
    }
}
