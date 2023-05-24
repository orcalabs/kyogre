use crate::*;
use error_stack::Result;
use tracing::{event, instrument, Level};
use trip_assembler::TripAssemblerError;

// Trips -> TripsPrecision
impl<L, T> From<StepWrapper<L, T, Trips>> for StepWrapper<L, T, TripsPrecision> {
    fn from(val: StepWrapper<L, T, Trips>) -> StepWrapper<L, T, TripsPrecision> {
        val.inherit(TripsPrecision::default())
    }
}

// Pending -> Trips
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, Trips> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, Trips> {
        val.inherit(Trips::default())
    }
}

#[derive(Default)]
pub struct Trips;

impl<A, B> StepWrapper<A, SharedState<B>, Trips>
where
    B: Database,
{
    #[instrument(name = "trips_state", skip_all, fields(app.engine_state))]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record("app.engine_state", EngineDiscriminants::Trips.as_ref());
        for a in self.trip_processors() {
            if let Err(e) = self.run_assembler(a.as_ref()).await {
                event!(Level::ERROR, "failed to run trip assembler: {:?}", e);
            }
        }
        Engine::TripsPrecision(StepWrapper::<A, SharedState<B>, TripsPrecision>::from(self))
    }

    #[instrument(skip_all, fields(app.trip_assembler_id, app.vessels, app.produced_trips, app.conflicts, app.no_prior_state))]
    async fn run_assembler(
        &self,
        trip_assembler: &dyn TripProcessor,
    ) -> Result<(), TripAssemblerError> {
        let database = self.database();

        match trip_assembler.produce_and_store_trips(database).await {
            Ok(r) => {
                tracing::Span::current().record(
                    "app.trip_assembler",
                    trip_assembler.assembler_id().to_string(),
                );
                tracing::Span::current().record("app.vessels", r.num_vessels);
                tracing::Span::current().record("app.conflicts", r.num_conflicts);
                tracing::Span::current().record("app.no_prior_state", r.num_no_prior_state);
                tracing::Span::current().record("app.produced_trips", r.num_trips);
            }
            Err(e) => event!(Level::ERROR, "failed to produce and store trips: {:?}", e),
        }

        Ok(())
    }
}
