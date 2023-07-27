use crate::*;
use tracing::{event, instrument, Level};

// Pending -> TripDistance
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, TripDistance> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, TripDistance> {
        val.inherit(TripDistance)
    }
}

// TripDistance -> UpdateDatabaseViews
impl<L, T> From<StepWrapper<L, T, TripDistance>> for StepWrapper<L, T, UpdateDatabaseViews> {
    fn from(val: StepWrapper<L, T, TripDistance>) -> StepWrapper<L, T, UpdateDatabaseViews> {
        val.inherit(UpdateDatabaseViews)
    }
}

#[derive(Default)]
pub struct TripDistance;

impl<A, B> StepWrapper<A, SharedState<B>, TripDistance>
where
    B: Database,
{
    #[instrument(name = "trip_distance_state", skip_all, fields(app.engine_state))]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record(
            "app.engine_state",
            EngineDiscriminants::TripDistance.as_ref(),
        );
        self.do_step().await;
        Engine::UpdateDatabaseViews(StepWrapper::<A, SharedState<B>, UpdateDatabaseViews>::from(
            self,
        ))
    }

    #[instrument(skip_all)]
    async fn do_step(&self) {
        let database = self.database();
        for t in self.trip_distancers() {
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
}
