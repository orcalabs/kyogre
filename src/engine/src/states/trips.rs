use crate::*;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use error_stack::{Result, ResultExt};
use kyogre_core::{Vessel, VesselIdentificationId};
use tracing::{event, instrument, Level};
use trip_assembler::{State, TripAssemblerError};

// Trips -> UpdateDatabaseViews
impl<L, T> From<StepWrapper<L, T, Trips>> for StepWrapper<L, T, UpdateDatabaseViews> {
    fn from(val: StepWrapper<L, T, Trips>) -> StepWrapper<L, T, UpdateDatabaseViews> {
        val.inherit(UpdateDatabaseViews::default())
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
        Engine::UpdateDatabaseViews(StepWrapper::<A, SharedState<B>, UpdateDatabaseViews>::from(
            self,
        ))
    }

    #[instrument(name = "run_assembler", skip_all, fields(app.trip_assembler, app.vessels, app.conflicts,app.no_prior_state))]
    async fn run_assembler(
        &self,
        trip_assembler: &dyn TripProcessor,
    ) -> Result<(), TripAssemblerError> {
        let trip_assembler_id = trip_assembler.assembler_id();
        tracing::Span::current().record("app.trip_assembler", trip_assembler_id.to_string());
        let database = self.database();

        let timers = database
            .trip_calculation_timers(trip_assembler.assembler_id())
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|t| (t.vessel_identification_id, t.timestamp))
            .collect::<HashMap<VesselIdentificationId, DateTime<Utc>>>();

        let conflicts = database
            .conflicts(trip_assembler.assembler_id())
            .await
            .change_context(TripAssemblerError)?
            .into_iter()
            .map(|t| (t.vessel_identification_id, t.timestamp))
            .collect::<HashMap<VesselIdentificationId, DateTime<Utc>>>();

        let vessels = database
            .vessels()
            .await
            .change_context(TripAssemblerError)?;

        let mut vessel_states = Vec::new();

        let num_vessels = vessels.len();
        let mut num_conflicts = 0;
        let mut num_no_prior_state = 0;

        for vessel in vessels {
            if let Some(conflict) = conflicts.get(&vessel.id) {
                num_conflicts += 1;
                let prior_trip = database
                    .trip_at_or_prior_to(vessel.id, trip_assembler_id, conflict)
                    .await
                    .change_context(TripAssemblerError)?;
                vessel_states.push((
                    vessel,
                    State::Conflict {
                        conflict_timestamp: *conflict,
                        trip_prior_to_or_at_conflict: prior_trip,
                    },
                ));
            } else if let Some(timer) = timers.get(&vessel.id) {
                vessel_states.push((vessel, State::CurrentCalculationTime(*timer)));
            } else {
                num_no_prior_state += 1;
                vessel_states.push((vessel, State::NoPriorState));
            }
        }

        tracing::Span::current().record("app.vessels", num_vessels);
        tracing::Span::current().record("app.conflicts", num_conflicts);
        tracing::Span::current().record("app.no_prior_state", num_no_prior_state);

        event!(Level::INFO, "starting..",);

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
                    trips.new_trip_calculation_time,
                    trips.conflict_strategy,
                    trips.trips,
                    trip_assembler.assembler_id(),
                )
                .await
                .change_context(TripAssemblerError)?;
        }

        Ok(())
    }
}
