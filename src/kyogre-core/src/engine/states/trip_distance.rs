use crate::*;
use async_trait::async_trait;
use machine::Schedule;
use tracing::{event, Level};

pub struct TripsDistanceState;

#[async_trait]
impl machine::State for TripsDistanceState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        for t in &shared_state.trip_distancers {
            if let Err(e) = t
                .calculate_trips_distance(
                    shared_state.trip_distancer_inbound.as_ref(),
                    shared_state.trip_distancer_outbound.as_ref(),
                )
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to run trip distancer {}, err: {:?}",
                    t.trip_distancer_id(),
                    e
                );
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}
