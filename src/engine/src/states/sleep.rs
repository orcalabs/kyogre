use crate::{Engine, Pending, SharedState, StepWrapper};
use tracing::{event, Level};

// Pending -> Sleep
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, Sleep> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, Sleep> {
        let duration = match val.inner.state.sleep_duration {
            Some(s) => s,
            None => tokio::time::Duration::from_secs(10),
        };

        val.inherit(Sleep {
            sleep_duration: duration,
        })
    }
}

// Sleep -> Pending
impl<L, T> From<StepWrapper<L, T, Sleep>> for StepWrapper<L, T, Pending> {
    fn from(val: StepWrapper<L, T, Sleep>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

pub struct Sleep {
    sleep_duration: tokio::time::Duration,
}

impl<A, B> StepWrapper<A, SharedState<B>, Sleep> {
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        event!(
            Level::INFO,
            "sleeping {:?}",
            self.inner.state.sleep_duration
        );
        tokio::time::sleep(self.inner.state.sleep_duration).await;
        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }
}
