#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use haul_distributor::*;
use kyogre_core::*;
use orca_statemachine::{Machine, Schedule, Step, TransitionLog};
use scraper::Scraper;
use serde::Deserialize;
use states::{Pending, Scrape, Sleep, Trips, TripsPrecision};
use strum_macros::{AsRefStr, EnumDiscriminants, EnumIter, EnumString};
use trip_assembler::TripAssembler;
use trip_distancer::*;
use vessel_benchmark::*;

pub mod error;
pub mod settings;
pub mod startup;
pub mod states;

pub use error::*;
pub use settings::*;
pub use startup::*;
pub use states::*;

pub trait MatrixCache: MatrixCacheInbound + Send + Sync + 'static {}
impl<T> MatrixCache for T where T: MatrixCacheInbound + 'static {}

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
        + 'static
{
}
impl<T> TripProcessor for T where T: TripAssembler + 'static {}

#[derive(EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumString, EnumIter))]
pub enum Engine<A, B> {
    Pending(StepWrapper<A, B, Pending>),
    Sleep(StepWrapper<A, B, Sleep>),
    Scrape(StepWrapper<A, B, Scrape>),
    Trips(StepWrapper<A, B, Trips>),
    TripsPrecision(StepWrapper<A, B, TripsPrecision>),
    Benchmark(StepWrapper<A, B, Benchmark>),
    HaulDistribution(StepWrapper<A, B, HaulDistribution>),
    TripDistance(StepWrapper<A, B, TripDistance>),
}

pub struct StepWrapper<A, B, C> {
    pub inner: Step<A, B, C>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub scrape_schedule: Schedule,
}

pub struct SharedState<A, B> {
    config: Config,
    database: A,
    matrix_cache: B,
    scraper: Scraper,
    trip_processors: Vec<Box<dyn TripProcessor>>,
    benchmarks: Vec<Box<dyn VesselBenchmark>>,
    haul_distributors: Vec<Box<dyn HaulDistributor>>,
    trip_distancers: Vec<Box<dyn TripDistancer>>,
}

impl<A, B, C> StepWrapper<A, B, C> {
    pub fn initial(log: A, shared_state: B, state: C) -> StepWrapper<A, B, C> {
        StepWrapper {
            inner: Step::initial(state, shared_state, log),
        }
    }

    pub fn inherit<D>(self, state: D) -> StepWrapper<A, B, D> {
        StepWrapper {
            inner: self.inner.inherit(state),
        }
    }
}

impl<A, B, C, D> StepWrapper<A, SharedState<B, D>, C> {
    pub fn scraper(&self) -> &Scraper {
        &self.inner.shared_state.scraper
    }
    pub fn trip_processors(&self) -> &[Box<dyn TripProcessor>] {
        self.inner.shared_state.trip_processors.as_slice()
    }
    pub fn database(&self) -> &B {
        &self.inner.shared_state.database
    }
    pub fn matrix_cache(&self) -> &D {
        &self.inner.shared_state.matrix_cache
    }
    pub fn vessel_benchmarks(&self) -> &[Box<dyn VesselBenchmark>] {
        self.inner.shared_state.benchmarks.as_slice()
    }
    pub fn haul_distributors(&self) -> &[Box<dyn HaulDistributor>] {
        self.inner.shared_state.haul_distributors.as_slice()
    }
    pub fn trip_distancers(&self) -> &[Box<dyn TripDistancer>] {
        self.inner.shared_state.trip_distancers.as_slice()
    }
}

#[async_trait]
impl<A, B, C> Machine<A> for Engine<A, SharedState<B, C>>
where
    A: TransitionLog + Send + Sync + 'static,
    B: Database,
    C: MatrixCache,
{
    type SharedState = SharedState<B, C>;

    async fn step(self) -> Self {
        match self {
            Engine::Pending(s) => s.run().await,
            Engine::Sleep(s) => s.run().await,
            Engine::Scrape(s) => s.run().await,
            Engine::Trips(s) => s.run().await,
            Engine::TripsPrecision(s) => s.run().await,
            Engine::Benchmark(s) => s.run().await,
            Engine::HaulDistribution(s) => s.run().await,
            Engine::TripDistance(s) => s.run().await,
        }
    }
    fn is_exit_state(&self) -> bool {
        false
    }

    fn transition_log(&self) -> &A {
        match self {
            Engine::Pending(s) => &s.inner.transition_log,
            Engine::Sleep(s) => &s.inner.transition_log,
            Engine::Scrape(s) => &s.inner.transition_log,
            Engine::Trips(s) => &s.inner.transition_log,
            Engine::TripsPrecision(s) => &s.inner.transition_log,
            Engine::Benchmark(s) => &s.inner.transition_log,
            Engine::HaulDistribution(s) => &s.inner.transition_log,
            Engine::TripDistance(s) => &s.inner.transition_log,
        }
    }

    fn initial(shared_state: SharedState<B, C>, log: A) -> Self {
        Engine::Pending(StepWrapper::initialize(shared_state, log))
    }

    fn current_state_name(&self) -> String {
        EngineDiscriminants::from(self).as_ref().to_string()
    }
}

impl<A, B> SharedState<A, B>
where
    A: Database,
    B: MatrixCache,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: Config,
        database: A,
        matrix_cache: B,
        scraper: Scraper,
        trip_processors: Vec<Box<dyn TripProcessor>>,
        benchmarks: Vec<Box<dyn VesselBenchmark>>,
        haul_distributors: Vec<Box<dyn HaulDistributor>>,
        trip_distancers: Vec<Box<dyn TripDistancer>>,
    ) -> SharedState<A, B> {
        SharedState {
            config,
            database,
            scraper,
            trip_processors,
            benchmarks,
            haul_distributors,
            trip_distancers,
            matrix_cache,
        }
    }
}

impl Config {
    pub fn schedule(&self, state: &EngineDiscriminants) -> Option<&Schedule> {
        match state {
            EngineDiscriminants::Pending
            | EngineDiscriminants::TripDistance
            | EngineDiscriminants::HaulDistribution
            | EngineDiscriminants::Benchmark
            | EngineDiscriminants::Sleep
            | EngineDiscriminants::Trips
            | EngineDiscriminants::TripsPrecision => None,
            EngineDiscriminants::Scrape => Some(&self.scrape_schedule),
        }
    }
}
