use tracing::instrument;

use crate::{Engine, SharedState, StepWrapper};

use super::TripsPrecision;

// Trips -> TripsPrecision
impl<L, T> From<StepWrapper<L, T, Trips>> for StepWrapper<L, T, TripsPrecision> {
    fn from(val: StepWrapper<L, T, Trips>) -> StepWrapper<L, T, TripsPrecision> {
        val.inherit(TripsPrecision::default())
    }
}

#[derive(Default)]
pub struct Trips;

impl<A, B> StepWrapper<A, SharedState<B>, Trips> {
    #[instrument(name = "trips_state", skip_all)]
    pub fn run(self) -> Engine<A, SharedState<B>> {
        Engine::TripsPrecision(StepWrapper::<A, SharedState<B>, TripsPrecision>::from(self))
    }
}
