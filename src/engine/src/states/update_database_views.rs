use crate::*;
use tracing::{event, instrument, Level};

// Pending -> UpdateDatabaseViews
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, UpdateDatabaseViews> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, UpdateDatabaseViews> {
        val.inherit(UpdateDatabaseViews::default())
    }
}

// UpdateDatabaseViews -> Pending
impl<L, T> From<StepWrapper<L, T, UpdateDatabaseViews>> for StepWrapper<L, T, Pending> {
    fn from(val: StepWrapper<L, T, UpdateDatabaseViews>) -> StepWrapper<L, T, Pending> {
        val.inherit(Pending::default())
    }
}

#[derive(Default)]
pub struct UpdateDatabaseViews;

impl<A, B> StepWrapper<A, SharedState<B>, UpdateDatabaseViews>
where
    B: Database,
{
    #[instrument(name = "update_database_views_state", skip_all)]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record(
            "app.engine_state",
            EngineDiscriminants::UpdateDatabaseViews.as_ref(),
        );
        if let Err(e) = self.database().update_database_views().await {
            event!(
                Level::ERROR,
                "failed to process update_database_views_state, error: {e:?}"
            );
        };

        Engine::Pending(StepWrapper::<A, SharedState<B>, Pending>::from(self))
    }
}
