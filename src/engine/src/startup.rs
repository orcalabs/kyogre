use crate::{settings::Settings, Engine, SharedState};
use orca_statemachine::Machine;
use postgres::PostgresAdapter;
use scraper::Scraper;

pub struct App {
    shared_state: SharedState<PostgresAdapter>,
    transition_log: orca_statemachine::Client,
}

impl App {
    pub async fn build(settings: &Settings) -> App {
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();
        let scraper = Scraper::new(settings.scraper.clone(), Box::new(postgres.clone()));
        let trip_assemblers = settings.trip_assemblers();
        let transition_log = orca_statemachine::Client::new(&settings.postgres)
            .await
            .unwrap();

        let shared_state =
            SharedState::new(settings.engine.clone(), postgres, scraper, trip_assemblers);

        App {
            transition_log,
            shared_state,
        }
    }

    pub async fn run(self) {
        Engine::run(self.shared_state, self.transition_log, None).await;
    }
}
