use tracing::instrument;

use crate::{Engine, Pending, SharedState, StepWrapper, UpdateDatabaseViews};

// Pending -> Scrape
impl<L, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, Scrape> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, Scrape> {
        val.inherit(Scrape::default())
    }
}

// Scrape -> UpdateDatabaseViews
impl<L, T> From<StepWrapper<L, T, Scrape>> for StepWrapper<L, T, UpdateDatabaseViews> {
    fn from(val: StepWrapper<L, T, Scrape>) -> StepWrapper<L, T, UpdateDatabaseViews> {
        val.inherit(UpdateDatabaseViews::default())
    }
}

#[derive(Default)]
pub struct Scrape;

impl<A, B> StepWrapper<A, SharedState<B>, Scrape> {
    #[instrument(name = "scrape_state", skip_all)]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        self.scraper().run().await;
        Engine::UpdateDatabaseViews(StepWrapper::<A, SharedState<B>, UpdateDatabaseViews>::from(
            self,
        ))
    }
}
