use crate::{SharedState, TripAssembler, TripPrecisionInboundPort};
use crate::{TripPrecisionError, TripPrecisionOutboundPort, Vessel};
use async_trait::async_trait;
use error_stack::ResultExt;
use machine::Schedule;
use tracing::{event, instrument, Level};

pub struct TripsPrecisionState;

#[async_trait]
impl machine::State for TripsPrecisionState {
    type SharedState = SharedState;

    async fn run(&self, shared_state: &Self::SharedState) {
        match shared_state
            .trips_precision_outbound_port
            .all_vessels()
            .await
        {
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
    read_storage: &dyn TripPrecisionOutboundPort,
    write_storage: &dyn TripPrecisionInboundPort,
    processor: &dyn TripAssembler,
    vessels: &[Vessel],
) -> error_stack::Result<(), TripPrecisionError> {
    tracing::Span::current().record("app.trip_assembler", processor.assembler_id().to_string());

    for vessel in vessels {
        if vessel.mmsi().is_none() && vessel.fiskeridir.call_sign.is_none() {
            continue;
        }

        let trips = read_storage
            .trips_without_precision(vessel.fiskeridir.id, processor.assembler_id())
            .await
            .change_context(TripPrecisionError)?;

        if trips.is_empty() {
            continue;
        }

        match processor
            .calculate_precision(vessel, read_storage, trips)
            .await
        {
            Ok(updates) => {
                write_storage
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
    for processor in &shared_state.trip_assemblers {
        if let Err(e) = run_precision_processor(
            shared_state.trips_precision_outbound_port.as_ref(),
            shared_state.trips_precision_inbound_port.as_ref(),
            processor.as_ref(),
            &vessels,
        )
        .await
        {
            event!(
                Level::ERROR,
                "failed to run trips_precision assembler, error: {:?}",
                e
            );
        }
    }
}
