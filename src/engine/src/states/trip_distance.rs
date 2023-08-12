use crate::*;
use async_trait::async_trait;
use tracing::{event, Level};

pub struct TripsDistanceState;

#[async_trait]
impl machine::State for TripsDistanceState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        let database = shared_state.postgres_adapter();
        for t in &shared_state.trip_distancers {
            if let Err(e) = t.calculate_trips_distance(database, database).await {
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
