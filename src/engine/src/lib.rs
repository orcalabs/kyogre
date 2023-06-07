#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use kyogre_core::*;
use orca_statemachine::{Machine, Schedule, Step, TransitionLog};
use scraper::Scraper;
use serde::Deserialize;
use states::{Pending, Scrape, Sleep, Trips, TripsPrecision};
use strum_macros::{AsRefStr, EnumDiscriminants, EnumIter, EnumString};
use trip_assembler::TripAssembler;
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
    + Send
    + Sync
    + 'static
{
}
pub trait TripProcessor: TripAssembler + Send + Sync + 'static {}
// pub trait VesselBenchmark: vessel_benchmark::VesselBenchmark + Send + Sync + 'static {}

impl<T> Database for T where
    T: TripAssemblerOutboundPort
        + TripPrecisionInboundPort
        + TripPrecisionOutboundPort
        + ScraperInboundPort
        + VesselBenchmarkOutbound
        + VesselBenchmarkInbound
        + 'static
{
}
impl<T> TripProcessor for T where T: TripAssembler + 'static {}
// impl<T> VesselBenchmark for T where T: vessel_benchmark::VesselBenchmark + 'static {}

#[derive(EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumString, EnumIter))]
pub enum Engine<A, B> {
    Pending(StepWrapper<A, B, Pending>),
    Sleep(StepWrapper<A, B, Sleep>),
    Scrape(StepWrapper<A, B, Scrape>),
    Trips(StepWrapper<A, B, Trips>),
    TripsPrecision(StepWrapper<A, B, TripsPrecision>),
    Benchmark(StepWrapper<A, B, Benchmark>),
}

pub struct StepWrapper<A, B, C> {
    pub inner: Step<A, B, C>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub scrape_schedule: Schedule,
}

pub struct SharedState<A> {
    config: Config,
    database: A,
    scraper: Scraper,
    trip_processors: Vec<Box<dyn TripProcessor>>,
    benchmarks: Vec<Box<dyn VesselBenchmark>>,
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
}

#[async_trait]
impl<A, B> Machine<A> for Engine<A, SharedState<B>>
where
    A: TransitionLog + Send + Sync + 'static,
    B: Database,
{
    type SharedState = SharedState<B>;

    async fn step(self) -> Self {
        match self {
            Engine::Pending(s) => s.run().await,
            Engine::Sleep(s) => s.run().await,
            Engine::Scrape(s) => s.run().await,
            Engine::Trips(s) => s.run().await,
            Engine::TripsPrecision(s) => s.run().await,
            Engine::Benchmark(s) => s.run().await,
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
        }
    }

    fn initial(shared_state: SharedState<B>, log: A) -> Self {
        Engine::Pending(StepWrapper::initialize(shared_state, log))
    }

    fn current_state_name(&self) -> String {
        EngineDiscriminants::from(self).as_ref().to_string()
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
    ) -> SharedState<A> {
        SharedState {
            config,
            database,
            scraper,
            trip_processors,
            benchmarks,
        }
    }
}

impl Config {
    pub fn schedule(&self, state: &EngineDiscriminants) -> Option<&Schedule> {
        match state {
            EngineDiscriminants::Pending
            | EngineDiscriminants::Benchmark
            | EngineDiscriminants::Sleep
            | EngineDiscriminants::Trips
            | EngineDiscriminants::TripsPrecision => None,
            EngineDiscriminants::Scrape => Some(&self.scrape_schedule),
        }
    }
}
