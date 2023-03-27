use std::str::FromStr;

use crate::{
    error::EngineError, Engine, EngineDiscriminants, Scrape, SharedState, Sleep, StepWrapper,
    UpdateDatabaseViews,
};

use chrono::{DateTime, Utc};
use error_stack::{report, IntoReport, Result, ResultExt};
use orca_statemachine::TransitionLog;
use strum::IntoEnumIterator;
use tracing::{event, instrument, Level};

#[derive(Default)]
pub struct Pending {
    pub(crate) sleep_duration: Option<tokio::time::Duration>,
}

struct LastTranstion {
    state: EngineDiscriminants,
    time: Option<DateTime<Utc>>,
}

enum NextState {
    State(EngineDiscriminants),
    Sleep(std::time::Duration),
}

impl<A, B> StepWrapper<A, SharedState<B>, Pending>
where
    A: TransitionLog + Send + Sync + 'static,
{
    pub fn initialize(shared: SharedState<B>, log: A) -> StepWrapper<A, SharedState<B>, Pending> {
        StepWrapper::initial(log, shared, Pending::default())
    }

    #[instrument(name = "pending_state", skip_all)]
    pub async fn run(mut self) -> Engine<A, SharedState<B>> {
        match self.next_transition().await {
            Ok(s) => {
                event!(Level::INFO, "next state is: {:?}", s);
                self.transition(s)
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

    async fn next_transition(&mut self) -> Result<EngineDiscriminants, EngineError> {
        let next_state = match self.check_for_interrupted_chain().await? {
            Some(state) => state,
            None => {
                let last_transitions = self.last_transitions().await?;
                let current_time = chrono::Utc::now();
                self.resolve_next_state(last_transitions, current_time)
            }
        };

        match next_state {
            NextState::State(s) => Ok(s),
            NextState::Sleep(d) => {
                self.inner.state.sleep_duration = Some(d);
                Ok(EngineDiscriminants::Sleep)
            }
        }
    }

    async fn check_for_interrupted_chain(&self) -> Result<Option<NextState>, EngineError> {
        self.inner
            .transition_log
            .last_in_chain(
                EngineDiscriminants::Scrape.as_ref(),
                EngineDiscriminants::UpdateDatabaseViews.as_ref(),
                EngineDiscriminants::Pending.as_ref(),
                10,
            )
            .await
            .map_err(|_| report!(EngineError::Transition))?
            .map(|state_name| {
                let state = EngineDiscriminants::from_str(&state_name)
                    .into_report()
                    .change_context(EngineError::Transition)?;
                Ok(NextState::State(state))
            })
            .transpose()
    }

    async fn last_transitions(&self) -> Result<Vec<LastTranstion>, EngineError> {
        let mut last_transitions = Vec::new();

        for s in EngineDiscriminants::iter() {
            match s {
                EngineDiscriminants::Sleep | EngineDiscriminants::Pending => continue,
                _ => (),
            }

            let last_transition_time: Option<DateTime<Utc>> = self
                .inner
                .transition_log
                .last_transition_of(s.as_ref())
                .await
                .map_err(|_| report!(EngineError::Transition))
                .map(|t| t.map(|d| d.date))?;

            last_transitions.push(LastTranstion {
                state: s,
                time: last_transition_time,
            });
        }

        Ok(last_transitions)
    }

    fn resolve_next_state(
        &self,
        last_transitions: Vec<LastTranstion>,
        current_time: DateTime<Utc>,
    ) -> NextState {
        let mut ready_states = Vec::new();
        let mut time_til_ready = Vec::new();
        for last_transition in last_transitions {
            if let Some(schedule) = self
                .inner
                .shared_state
                .config
                .schedule(&last_transition.state)
            {
                if schedule.should_schedule(current_time, last_transition.time) {
                    ready_states.push(last_transition);
                } else if let Some(duration) =
                    schedule.next_transition(current_time, last_transition.time)
                {
                    time_til_ready.push(duration);
                }
            }
        }

        if ready_states.is_empty() {
            if time_til_ready.is_empty() {
                event!(
                    Level::WARN,
                    "no states returned a schedule, are all states disabled? Sleeping a minute..."
                );
                NextState::Sleep(std::time::Duration::from_secs(60))
            } else {
                NextState::Sleep(*time_til_ready.iter().min().unwrap())
            }
        } else {
            let states_without_prior_transition: Vec<&LastTranstion> =
                ready_states.iter().filter(|s| s.time.is_none()).collect();

            if !states_without_prior_transition.is_empty() {
                NextState::State(states_without_prior_transition[0].state)
            } else {
                NextState::State(
                    ready_states
                        .into_iter()
                        .filter(|s| s.time.is_some())
                        .min_by_key(|s| s.time)
                        .unwrap()
                        .state,
                )
            }
        }
    }

    fn transition(self, new_state: EngineDiscriminants) -> Engine<A, SharedState<B>> {
        match new_state {
            EngineDiscriminants::Trips | EngineDiscriminants::TripsPrecision => {
                panic!("tried to enter the Trips/TripsPrecision state from the Pending state")
            }
            EngineDiscriminants::Pending => {
                panic!("tried to enter the Pending state from the Pending state")
            }
            EngineDiscriminants::Sleep => {
                Engine::Sleep(StepWrapper::<A, SharedState<B>, Sleep>::from(self))
            }
            EngineDiscriminants::Scrape => {
                Engine::Scrape(StepWrapper::<A, SharedState<B>, Scrape>::from(self))
            }
            EngineDiscriminants::UpdateDatabaseViews => {
                Engine::UpdateDatabaseViews(
                    StepWrapper::<A, SharedState<B>, UpdateDatabaseViews>::from(self),
                )
            }
        }
    }
}
