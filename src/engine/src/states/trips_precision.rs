use crate::{SharedState, TripProcessor};
use async_trait::async_trait;
use error_stack::ResultExt;
use kyogre_core::{
    TripAssemblerOutboundPort, TripPrecisionInboundPort, TripPrecisionOutboundPort, Vessel,
};
use machine::Schedule;
use postgres::PostgresAdapter;
use tracing::{event, instrument, Level};
use trip_assembler::TripPrecisionError;

pub struct TripsPrecisionState;

#[async_trait]
impl machine::State for TripsPrecisionState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        match shared_state.database.all_vessels().await {
            Err(e) => {
                event!(Level::ERROR, "failed to retrieve vessels: {:?}", e);
            }
            Ok(vessels) => {
                run_precision_processors(shared_state, vessels).await;
            }
        }
    }
    fn schedule(&self) -> Schedule {
        Schedule::Disabled
    }
}

#[instrument(name = "run_precision_assembler", skip_all, fields(app.trip_assembler))]
async fn run_precision_processor(
    database: &PostgresAdapter,
    processor: &dyn TripProcessor,
    vessels: &[Vessel],
) -> error_stack::Result<(), TripPrecisionError> {
    tracing::Span::current().record("app.trip_assembler", processor.assembler_id().to_string());

    for vessel in vessels {
        if vessel.mmsi().is_none() && vessel.fiskeridir.call_sign.is_none() {
            continue;
        }

        let trips = database
            .trips_without_precision(vessel.fiskeridir.id, processor.assembler_id())
            .await
            .change_context(TripPrecisionError)?;

        if trips.is_empty() {
            continue;
        }

        match processor.calculate_precision(vessel, database, trips).await {
            Ok(updates) => {
                database
                    .update_trip_precisions(updates)
                    .await
                    .change_context(TripPrecisionError)?;
            }
            Err(e) => {
                event!(Level::ERROR, "failed to calculate trips precision: {:?}", e);
            }
        }
    }

    Ok(())
}

async fn run_precision_processors(shared_state: &SharedState, vessels: Vec<Vessel>) {
    let database = shared_state.postgres_adapter();
    for processor in &shared_state.trip_processors {
        if let Err(e) = run_precision_processor(database, processor.as_ref(), &vessels).await {
            event!(
                Level::ERROR,
                "failed to run trips_precision assembler, error: {:?}",
                e
            );
        }
    }
}
