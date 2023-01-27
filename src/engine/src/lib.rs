#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use orca_statemachine::{Machine, Schedule, Step, TransitionLog};
use scraper::Scraper;
use serde::Deserialize;
use states::{Pending, Scrape, Sleep, Trips, TripsPrecision};
use strum_macros::{AsRefStr, EnumDiscriminants, EnumIter, EnumString};

pub mod error;
pub mod settings;
pub mod startup;
pub mod states;

#[derive(EnumDiscriminants)]
#[strum_discriminants(derive(AsRefStr, EnumString, EnumIter))]
pub enum Engine<A, B> {
    Pending(StepWrapper<A, B, Pending>),
    Sleep(StepWrapper<A, B, Sleep>),
    Scrape(StepWrapper<A, B, Scrape>),
    Trips(StepWrapper<A, B, Trips>),
    TripsPrecision(StepWrapper<A, B, TripsPrecision>),
}

pub struct StepWrapper<A, B, C> {
    pub inner: Step<A, B, C>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    scrape_schedule: Schedule,
}

#[allow(dead_code)]
pub struct SharedState<A> {
    config: Config,
    database: A,
    scraper: Scraper,
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
}

#[async_trait]
impl<A, B> Machine<A> for Engine<A, SharedState<B>>
where
    A: TransitionLog + Send + Sync + 'static,
    B: Send + Sync + 'static,
{
    type SharedState = SharedState<B>;

    async fn step(self) -> Self {
        match self {
            Engine::Pending(s) => s.run().await,
            Engine::Sleep(s) => s.run().await,
            Engine::Scrape(s) => s.run().await,
            Engine::Trips(s) => s.run(),
            Engine::TripsPrecision(s) => s.run(),
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
        }
    }

    fn initial(shared_state: SharedState<B>, log: A) -> Self {
        Engine::Pending(StepWrapper::initialize(shared_state, log))
    }

    fn current_state_name(&self) -> String {
        EngineDiscriminants::from(self).as_ref().to_string()
    }
}

impl<A> SharedState<A> {
    pub fn new(config: Config, database: A, scraper: Scraper) -> SharedState<A> {
        SharedState {
            config,
            database,
            scraper,
        }
    }
}

impl Config {
    pub fn schedule(&self, state: &EngineDiscriminants) -> Option<&Schedule> {
        match state {
            EngineDiscriminants::Pending
            | EngineDiscriminants::Sleep
            | EngineDiscriminants::Trips
            | EngineDiscriminants::TripsPrecision => None,
            EngineDiscriminants::Scrape => Some(&self.scrape_schedule),
        }
    }
}
