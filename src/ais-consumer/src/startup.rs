use crate::{
    barentswatch::BarentswatchAisClient, consumer::Consumer, error::ConsumerError,
    settings::Settings,
};
use error_stack::Result;
use hyper::Uri;
use kyogre_core::{BearerToken, DataMessage};
use orca_core::Environment;
use postgres::PostgresAdapter;
use std::str::FromStr;
use tokio::{
    io::AsyncRead,
    sync::broadcast::{self, Receiver, Sender},
};

pub struct App {
    consumer: Consumer,
    postgres: PostgresAdapter,
    sender: Sender<DataMessage>,
    ais_source: Option<BarentswatchAisClient>,
}

impl App {
    pub async fn build(settings: Settings) -> App {
        let (sender, _) = broadcast::channel::<DataMessage>(settings.broadcast_buffer_size);
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        let ais_source = if let Environment::Test = settings.environment {
            None
        } else {
            let bearer_token = BearerToken::acquire(&settings.oauth.unwrap())
                .await
                .unwrap();
            let uri = Uri::from_str(&settings.api_address.unwrap()).unwrap();
            Some(BarentswatchAisClient::new(bearer_token, uri))
        };

        if settings.environment == Environment::Local {
            postgres.do_migrations().await;
        }

        App {
            postgres,
            sender,
            consumer: Consumer::new(settings.commit_interval),
            ais_source,
        }
    }

    pub fn subscribe(&self) -> Receiver<DataMessage> {
        self.sender.subscribe()
    }

    pub async fn run(self) {
        let receiver = self.subscribe();
        tokio::spawn(self.postgres.consume_loop(receiver, None));
        self.consumer
            .run(
                self.ais_source.unwrap().streamer().await.unwrap(),
                self.sender,
            )
            .await
            .unwrap()
    }

    pub async fn run_test(
        self,
        source: impl AsyncRead + Unpin,
        postgres_process_confirmation: tokio::sync::mpsc::Sender<()>,
    ) -> Result<(), ConsumerError> {
        let receiver = self.subscribe();
        tokio::spawn(
            self.postgres
                .consume_loop(receiver, Some(postgres_process_confirmation)),
        );
        self.consumer.run(source, self.sender).await
    }
}
