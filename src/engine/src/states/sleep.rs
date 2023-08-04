use crate::*;
use orca_statemachine::Pending;
use tracing::{event, instrument, Level};

// Pending -> Sleep
impl<L: TransitionLog, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, Sleep> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, Sleep> {
        let duration = match val.inner.state.sleep_duration {
            Some(s) => s.to_std().unwrap(),
            None => tokio::time::Duration::from_secs(10),
        };

        val.inherit(Sleep {
            sleep_duration: duration,
        })
    }
}

impl Default for Sleep {
    fn default() -> Self {
        Sleep {
            sleep_duration: tokio::time::Duration::from_millis(10),
        }
    }
}

// Sleep -> Pending
impl<L: TransitionLog, T> From<StepWrapper<L, T, Sleep>> for StepWrapper<L, T, Pending> {
    fn from(val: StepWrapper<L, T, Sleep>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

pub struct Sleep {
    pub sleep_duration: tokio::time::Duration,
}

impl<A: TransitionLog, B> StepWrapper<A, SharedState<B>, Sleep> {
    #[instrument(name = "sleep_state", skip_all)]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record("app.engine_state", EngineDiscriminants::Sleep.as_ref());
        event!(
            Level::INFO,
            "sleeping {:?}",
            self.inner.state.sleep_duration
        );
        tokio::time::sleep(self.inner.state.sleep_duration).await;
        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }
}
