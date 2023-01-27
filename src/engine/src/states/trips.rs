use std::collections::HashMap;

use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::Vessel;
use tracing::{event, instrument, Level};
use trip_assembler::{State, TripAssemblerError};

use crate::{Database, Engine, SharedState, StepWrapper, TripProcessor};

use super::TripsPrecision;

// Trips -> TripsPrecision
impl<L, T> From<StepWrapper<L, T, Trips>> for StepWrapper<L, T, TripsPrecision> {
    fn from(val: StepWrapper<L, T, Trips>) -> StepWrapper<L, T, TripsPrecision> {
        val.inherit(TripsPrecision::default())
    }
}

#[derive(Default)]
pub struct Trips;

impl<A, B> StepWrapper<A, SharedState<B>, Trips>
where
    B: Database,
{
    #[instrument(name = "trips_state", skip_all)]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        for a in self.trip_processors() {
            if let Err(e) = self.run_assembler(a.as_ref()).await {
                event!(Level::ERROR, "failed to run trip assembler: {:?}", e);
            }
        }
        Engine::TripsPrecision(StepWrapper::<A, SharedState<B>, TripsPrecision>::from(self))
    }

    #[instrument(name = "run_assembler", skip_all, fields(app.trip_assembler))]
    async fn run_assembler(
        &self,
        trip_assembler: &dyn TripProcessor,
    ) -> Result<(), TripAssemblerError> {
        tracing::Span::current().record(
            "app.trip_assembler",
            trip_assembler.assembler_id().to_string(),
        );
        let database = self.database();

        let timers = database
            .trip_calculation_timers()
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|t| (t.vessel_id, t.timestamp))
            .collect::<HashMap<i64, DateTime<Utc>>>();

        let conflicts = database
            .conflicts(trip_assembler.assembler_id())
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|t| (t.vessel_id, t.timestamp))
            .collect::<HashMap<i64, DateTime<Utc>>>();

        let vessels = database
            .vessels()
            .await
            .change_context(TripAssemblerError)?;

        let mut vessel_states = Vec::new();

        for vessel in vessels {
            if let Some(conflict) = conflicts.get(&vessel.id) {
                vessel_states.push((vessel, State::Conflict(*conflict)));
            } else if let Some(timer) = timers.get(&vessel.id) {
                vessel_states.push((vessel, State::CurrentCalculationTime(*timer)));
            } else {
                vessel_states.push((vessel, State::NoPriorState));
            }
        }

        for s in vessel_states {
            let id = s.0.id;
            if let Err(e) = self.run_assembler_on_vessel(trip_assembler, s.0, s.1).await {
                event!(
                    Level::ERROR,
                    "failed to run trip assembler for vessel: {:?}, err: {:?}",
                    id,
                    e
                );
            }
        }

        Ok(())
    }

    async fn run_assembler_on_vessel(
        &self,
        trip_assembler: &dyn TripProcessor,
        vessel: Vessel,
        state: State,
    ) -> Result<(), TripAssemblerError> {
        let database = self.database();
        let trips = trip_assembler.assemble(database, &vessel, state).await?;

        if let Some(trips) = trips {
            database
                .add_trips(
                    vessel.id,
                    trips.new_trip_calucation_time,
                    trips.conflict_strategy,
                    trips.trips,
                )
                .await
                .change_context(TripAssemblerError)?;
        }

        Ok(())
    }
}
