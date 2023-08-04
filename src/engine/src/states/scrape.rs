use crate::*;
use orca_statemachine::Pending;
use tracing::{event, instrument, Level};

// Pending -> Scrape
impl<L: TransitionLog, T> From<StepWrapper<L, T, Pending>> for StepWrapper<L, T, Scrape> {
    fn from(val: StepWrapper<L, T, Pending>) -> StepWrapper<L, T, Scrape> {
        val.inherit(Scrape)
    }
}

// Scrape -> Trips
impl<L: TransitionLog, T> From<StepWrapper<L, T, Scrape>> for StepWrapper<L, T, Trips> {
    fn from(val: StepWrapper<L, T, Scrape>) -> StepWrapper<L, T, Trips> {
        val.inherit(Trips)
    }
}

#[derive(Default)]
pub struct Scrape;

impl<A: TransitionLog, B: Database> StepWrapper<A, SharedState<B>, Scrape> {
    #[instrument(name = "scrape_state", skip_all)]
    pub async fn run(self) -> Engine<A, SharedState<B>> {
        tracing::Span::current().record("app.engine_state", EngineDiscriminants::Scrape.as_ref());
        self.scraper().run().await;
        if let Err(e) = self.database().increment().await {
            event!(Level::ERROR, "failed to queue matrix cache update: {:?}", e);
        }
        Engine::Trips(StepWrapper::<A, SharedState<B>, Trips>::from(self))
    }
}
