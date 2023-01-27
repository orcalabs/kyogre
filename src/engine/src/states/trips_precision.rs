use tracing::instrument;

use crate::{Engine, Pending, SharedState, StepWrapper};

// TripsPrecision -> Pending
impl<L, T> From<StepWrapper<L, T, TripsPrecision>> for StepWrapper<L, T, Pending> {
    fn from(val: StepWrapper<L, T, TripsPrecision>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

#[derive(Default)]
pub struct TripsPrecision;

impl<A, B> StepWrapper<A, SharedState<B>, TripsPrecision> {
    #[instrument(name = "trips_precision_state", skip_all)]
    pub fn run(self) -> Engine<A, SharedState<B>> {
        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }
}
