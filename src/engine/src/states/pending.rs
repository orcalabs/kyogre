use super::{Trips, TripsPrecision};
use crate::{
    Benchmark, Engine, EngineDiscriminants, HaulDistribution, Scrape, SharedState, Sleep,
    StepWrapper, TripDistance, UpdateDatabaseViews,
};
use chrono::Duration;
use orca_statemachine::{NextState, Pending, Schedule, StateChain, TransitionLog};
use strum::IntoEnumIterator;
use tracing::{event, instrument, Level};

impl<A: TransitionLog, B> StepWrapper<A, SharedState<B>, Pending> {
    #[instrument(name = "pending_state", skip_all)]
    pub async fn run(mut self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record("app.engine_state", EngineDiscriminants::Pending.as_ref());

        let states: Vec<(EngineDiscriminants, Schedule)> = EngineDiscriminants::iter()
            .map(|s| {
                let schedule = match s {
                    EngineDiscriminants::Pending => Schedule::Disabled,
                    EngineDiscriminants::Sleep => Schedule::Disabled,
                    EngineDiscriminants::Trips => Schedule::Disabled,
                    EngineDiscriminants::TripsPrecision => Schedule::Disabled,
                    EngineDiscriminants::Benchmark => Schedule::Disabled,
                    EngineDiscriminants::HaulDistribution => Schedule::Disabled,
                    EngineDiscriminants::TripDistance => Schedule::Disabled,
                    EngineDiscriminants::UpdateDatabaseViews => Schedule::Disabled,
                    EngineDiscriminants::Scrape => self.inner.shared_state.config.scrape_schedule,
                };
                (s, schedule)
            })
            .collect();

        let chains = vec![StateChain {
            starter: EngineDiscriminants::Scrape,
            end: EngineDiscriminants::UpdateDatabaseViews,
            breakpoint: EngineDiscriminants::Pending,
            max_lookback: 20,
        }];

        match self.inner.state.run(&self.inner.log, states, chains).await {
            Ok(s) => {
                let state = match s {
                    NextState::ReadyIn(d) => {
                        self.inner.state.sleep_duration = Some(d);
                        EngineDiscriminants::Sleep
                    }
                    NextState::NoSchedules => {
                        event!(Level::WARN, "no states returned a schedule, are all states disabled? Sleeping a minute...");
                        self.inner.state.sleep_duration = Some(Duration::seconds(60));
                        EngineDiscriminants::Sleep
                    }
                    NextState::State(s) => s,
                };
                event!(Level::INFO, "next state is: {:?}", state);
                self.transition(state)
            }
            Err(e) => {
                event!(
                    Level::ERROR,
                    "failed to decide upon the next state transition: {:?}, entering sleep state..",
                    e
                );
                Engine::Sleep(StepWrapper::<A, SharedState<B>, Sleep>::from(self))
            }
        }
    }

    fn transition(self, new_state: EngineDiscriminants) -> Engine<A, SharedState<B>> {
        match new_state {
            EngineDiscriminants::Pending => {
                panic!("tried to enter the Pending state from the Pending state")
            }
            EngineDiscriminants::Trips => {
                Engine::Trips(StepWrapper::<A, SharedState<B>, Trips>::from(self))
            }
            EngineDiscriminants::TripsPrecision => {
                Engine::TripsPrecision(StepWrapper::<A, SharedState<B>, TripsPrecision>::from(self))
            }
            EngineDiscriminants::Sleep => {
                Engine::Sleep(StepWrapper::<A, SharedState<B>, Sleep>::from(self))
            }
            EngineDiscriminants::Scrape => {
                Engine::Scrape(StepWrapper::<A, SharedState<B>, Scrape>::from(self))
            }
            EngineDiscriminants::Benchmark => {
                Engine::Benchmark(StepWrapper::<A, SharedState<B>, Benchmark>::from(self))
            }
            EngineDiscriminants::HaulDistribution => {
                Engine::HaulDistribution(StepWrapper::<A, SharedState<B>, HaulDistribution>::from(
                    self,
                ))
            }
            EngineDiscriminants::TripDistance => {
                Engine::TripDistance(StepWrapper::<A, SharedState<B>, TripDistance>::from(self))
            }
            EngineDiscriminants::UpdateDatabaseViews => {
                Engine::UpdateDatabaseViews(
                    StepWrapper::<A, SharedState<B>, UpdateDatabaseViews>::from(self),
                )
            }
        }
    }
}
