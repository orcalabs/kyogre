use std::str::FromStr;

use crate::{
    barentswatch::BarentswatchAisClient, consumer::Consumer, error::Result, settings::Settings,
};
use kyogre_core::{BearerToken, DataMessage};
use orca_core::Environment;
use postgres::PostgresAdapter;
use reqwest::Url;
use tokio::{
    io::AsyncRead,
    sync::broadcast::{self, Receiver, Sender},
};
use tracing::error;

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
            Some(BarentswatchAisClient::new(
                bearer_token,
                Url::from_str(&settings.api_address.unwrap()).unwrap(),
            ))
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
        tokio::spawn(async move { self.postgres.consume_loop(receiver, None).await });

        if let Err(e) = self
            .consumer
            .run(
                self.ais_source.unwrap().streamer().await.unwrap(),
                self.sender,
            )
            .await
        {
            error!("consumer failed: {e:?}");
        }
    }

    pub async fn run_test(
        self,
        source: impl AsyncRead + Unpin,
        postgres_process_confirmation: tokio::sync::mpsc::Sender<()>,
    ) -> Result<()> {
        let receiver = self.subscribe();
        tokio::spawn(async move {
            self.postgres
                .consume_loop(receiver, Some(postgres_process_confirmation))
                .await
        });
        self.consumer.run(source, self.sender).await
    }
}
