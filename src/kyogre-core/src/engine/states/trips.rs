use crate::{SharedState, TripAssembler, TripAssemblerError};
use async_trait::async_trait;
use machine::Schedule;
use tracing::{event, instrument, Level};

pub struct TripsState;

#[async_trait]
impl machine::State for TripsState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        for a in &shared_state.trip_assemblers {
            if let Err(e) = run_assembler(shared_state, a.as_ref()).await {
                event!(Level::ERROR, "failed to run trip assembler: {:?}", e);
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

#[instrument(skip_all, fields(app.trip_assembler_id))]
async fn run_assembler(
    shared_state: &SharedState,
    trip_assembler: &dyn TripAssembler,
) -> Result<(), TripAssemblerError> {
    match trip_assembler
        .produce_and_store_trips(shared_state.trip_assembler_outbound_port.as_ref())
        .await
    {
        Ok(r) => {
            event!(
                Level::INFO,
                "num_conflicts: {}, num_vessels: {}, num_no_prior_state: {}
                       num_trips: {}, num_failed: {}, num_reset: {}",
                r.num_conflicts,
                r.num_vessels,
                r.num_no_prior_state,
                r.num_trips,
                r.num_failed,
                r.num_reset
            );

            tracing::Span::current().record(
                "app.trip_assembler",
                trip_assembler.assembler_id().to_string(),
            );
        }
        Err(e) => event!(Level::ERROR, "failed to produce and store trips: {:?}", e),
    }

    Ok(())
}