use crate::*;
use orca_statemachine::Pending;
use tracing::{event, instrument, Level};

// Pending -> UpdateDatabaseViews
impl<L: TransitionLog, T> From<StepWrapper<L, T, Pending>>
    for StepWrapper<L, T, UpdateDatabaseViews>
{
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, UpdateDatabaseViews> {
        val.inherit(UpdateDatabaseViews)
    }
}

// UpdateDatabaseViews -> Pending
impl<L: TransitionLog, T> From<StepWrapper<L, T, UpdateDatabaseViews>>
    for StepWrapper<L, T, Pending>
{
    fn from(val: StepWrapper<L, T, UpdateDatabaseViews>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

#[derive(Default)]
pub struct UpdateDatabaseViews;

impl<A: TransitionLog, B: Database> StepWrapper<A, SharedState<B>, UpdateDatabaseViews> {
    #[instrument(name = "update_database_views", skip_all, fields(app.engine_state))]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record(
            "app.engine_state",
            EngineDiscriminants::UpdateDatabaseViews.as_ref(),
        );

        if let Err(e) = self.database().refresh().await {
            event!(Level::ERROR, "failed to update database views {:?}", e);
        }

        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }
}
