use core::panic;
use std::{str::FromStr, time::Duration};

use crate::{
    barentswatch::BarentswatchAisClient, consumer::Consumer, error::Result, settings::Settings,
};
use async_channel::{Receiver, Sender};
use kyogre_core::{DataMessage, IsTimeout};
use orca_core::Environment;
use postgres::PostgresAdapter;
use reqwest::Url;
use tokio::{io::AsyncRead, task::JoinSet};
use tracing::{info, info_span};

pub struct App {
    consumer: Consumer,
    postgres: PostgresAdapter,
    sender: Sender<DataMessage>,
    receiver: Receiver<DataMessage>,
    ais_source: Option<BarentswatchAisClient>,
}

impl App {
    pub async fn build(settings: Settings) -> App {
        let (sender, receiver) =
            async_channel::bounded::<DataMessage>(settings.broadcast_buffer_size);
        let postgres = PostgresAdapter::new(&settings.postgres).await.unwrap();

        let ais_source = if let Environment::Test = settings.environment {
            None
        } else {
            Some(BarentswatchAisClient::new(
                settings.oauth.unwrap(),
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
            receiver,
        }
    }

    pub async fn run(self) {
        let receiver = self.receiver.clone();
        let postgres = self.postgres.clone();

        let mut set = JoinSet::new();

        set.spawn(async move { postgres.consume_loop(receiver, None).await });
        set.spawn(async move {
            loop {
                if let Err(e) = self.run_impl().await {
                    let span = info_span!("ais_consumer_run");
                    let _guard = span.enter();
                    if e.is_timeout() {
                        info!("retryable error encountered: '{e:?}', retrying");
                    } else {
                        // We assume the error is unrecoverable and requires a restart
                        panic!("consume error: {e:?}");
                    }
                }
                // If the ais api is unresponsive we dont want to relentlessly spam it
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        let out = set.join_next().await;
        panic!(
            "incoming ais consume loop or ais postgres loop exited unexpectedly: {:?}",
            out
        );
    }

    async fn run_impl(&self) -> Result<()> {
        let ais_source = self.ais_source.as_ref().unwrap().streamer().await?;
        self.consumer.run(ais_source, self.sender.clone()).await
    }

    pub async fn run_test(
        self,
        source: impl AsyncRead + Unpin,
        postgres_process_confirmation: tokio::sync::mpsc::Sender<()>,
    ) -> Result<()> {
        let receiver = self.receiver.clone();
        tokio::spawn(async move {
            self.postgres
                .consume_loop(receiver, Some(postgres_process_confirmation))
                .await
        });
        self.consumer.run(source, self.sender).await
    }
}
