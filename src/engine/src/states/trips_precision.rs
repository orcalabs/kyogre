use error_stack::{Result, ResultExt};
use kyogre_core::Vessel;
use tracing::{event, instrument, Level};
use trip_assembler::TripPrecisionError;

use crate::{Database, Engine, Pending, SharedState, StepWrapper, TripProcessor};

// TripsPrecision -> Pending
impl<L, T> From<StepWrapper<L, T, TripsPrecision>> for StepWrapper<L, T, Pending> {
    fn from(val: StepWrapper<L, T, TripsPrecision>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

#[derive(Default)]
pub struct TripsPrecision;

impl<A, B> StepWrapper<A, SharedState<B>, TripsPrecision>
where
    B: Database,
{
    #[instrument(name = "trips_precision_state", skip_all)]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        match self.database().vessels().await {
            Err(e) => {
                event!(Level::ERROR, "failed to retrieve vessels: {:?}", e);
            }
            Ok(vessels) => {
                self.run_precision_processors(vessels).await;
            }
        }

        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }

    async fn run_precision_processors(&self, vessels: Vec<Vessel>) {
        for processor in self.trip_processors() {
            if let Err(e) = self
                .run_precision_processor(processor.as_ref(), &vessels)
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

    #[instrument(name = "run_precision_assembler", skip_all, fields(app.trip_assembler))]
    async fn run_precision_processor(
        &self,
        processor: &dyn TripProcessor,
        vessels: &[Vessel],
    ) -> Result<(), TripPrecisionError> {
        tracing::Span::current().record("app.trip_assembler", processor.assembler_id().to_string());
        let database = self.database();

        for vessel in vessels {
            if vessel.mmsi.is_none() {
                continue;
            }

            let trips = database
                .trips_without_precision(vessel.id, processor.assembler_id())
                .await
                .change_context(TripPrecisionError)?;

            if trips.is_empty() {
                continue;
            }

            let updates = processor
                .calculate_precision(vessel, database, trips)
                .await?;

            database
                .update_trip_precisions(updates)
                .await
                .change_context(TripPrecisionError)?;
        }

        Ok(())
    }
}