#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use haul_distributor::*;
use kyogre_core::*;
use orca_statemachine::{Machine, Schedule, State, Step, TransitionLog};
use scraper::Scraper;
use serde::Deserialize;
use states::{Scrape, Sleep, Trips, TripsPrecision};
use std::str::FromStr;
use strum_macros::{AsRefStr, EnumDiscriminants, EnumIter, EnumString, IntoStaticStr};
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
        + 'static
{
}
impl<T> TripProcessor for T where T: TripAssembler + 'static {}

#[derive(EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumString, EnumIter, IntoStaticStr))]
pub enum Engine<A, B> {
    Pending(StepWrapper<A, B, orca_statemachine::Pending>),
    Sleep(StepWrapper<A, B, Sleep>),
    Scrape(StepWrapper<A, B, Scrape>),
    Trips(StepWrapper<A, B, Trips>),
    TripsPrecision(StepWrapper<A, B, TripsPrecision>),
    Benchmark(StepWrapper<A, B, Benchmark>),
    HaulDistribution(StepWrapper<A, B, HaulDistribution>),
    TripDistance(StepWrapper<A, B, TripDistance>),
    UpdateDatabaseViews(StepWrapper<A, B, UpdateDatabaseViews>),
}

pub struct StepWrapper<A, B, C> {
    pub inner: Step<A, B, C>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub scrape_schedule: Schedule,
}

pub struct SharedState<A> {
    pub config: Config,
    database: A,
    scraper: Scraper,
    trip_processors: Vec<Box<dyn TripProcessor>>,
    benchmarks: Vec<Box<dyn VesselBenchmark>>,
    haul_distributors: Vec<Box<dyn HaulDistributor>>,
    trip_distancers: Vec<Box<dyn TripDistancer>>,
}

impl State for EngineDiscriminants {
    fn name(&self) -> &'static str {
        self.into()
    }
    fn from_name(name: String) -> Self {
        EngineDiscriminants::from_str(&name).unwrap()
    }
}

impl<A, B, C> StepWrapper<A, B, C>
where
    A: TransitionLog,
{
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

impl<A, B, C> StepWrapper<A, SharedState<B>, C> {
    pub fn scraper(&self) -> &Scraper {
        &self.inner.shared_state.scraper
    }
    pub fn trip_processors(&self) -> &[Box<dyn TripProcessor>] {
        self.inner.shared_state.trip_processors.as_slice()
    }
    pub fn database(&self) -> &B {
        &self.inner.shared_state.database
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
impl<A, B> Machine<A, EngineDiscriminants> for Engine<A, SharedState<B>>
where
    A: TransitionLog + Send + Sync + 'static,
    B: Database,
{
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
            Engine::UpdateDatabaseViews(s) => s.run().await,
        }
    }

    fn current_state_name(&self) -> &'static str {
        let discriminant: EngineDiscriminants = self.into();
        discriminant.name()
    }
}

impl<A> SharedState<A>
where
    A: Database,
{
    pub fn new(
        config: Config,
        database: A,
        scraper: Scraper,
        trip_processors: Vec<Box<dyn TripProcessor>>,
        benchmarks: Vec<Box<dyn VesselBenchmark>>,
        haul_distributors: Vec<Box<dyn HaulDistributor>>,
        trip_distancers: Vec<Box<dyn TripDistancer>>,
    ) -> SharedState<A> {
        SharedState {
            config,
            database,
            scraper,
            trip_processors,
            benchmarks,
            haul_distributors,
            trip_distancers,
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
            | EngineDiscriminants::UpdateDatabaseViews
            | EngineDiscriminants::Sleep
            | EngineDiscriminants::Trips
            | EngineDiscriminants::TripsPrecision => None,
            EngineDiscriminants::Scrape => Some(&self.scrape_schedule),
        }
    }
}
