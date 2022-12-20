use crate::{consumer::Consumer, settings::Settings};
use ais_core::DataMessage;
use tokio::{
    io::AsyncRead,
    sync::broadcast::{self, Receiver, Sender},
};

pub struct App {
    consumer: Consumer,
    sender: Sender<DataMessage>,
}

impl App {
    pub fn build(settings: Settings) -> App {
        let (sender, receiver) = broadcast::channel::<DataMessage>(60);
        let adapter = PostgresAdapter::new(&settings.postgres).await.unwrap();

        tokio::spawn(adapter.run(receiver));

        App {
            sender,
            consumer: Consumer {},
        }
    }

    pub fn subscribe(&self) -> Receiver<DataMessage> {
        self.sender.subscribe()
    }

    pub async fn run(self, source: impl AsyncRead + Unpin) {
        self.consumer.run(source, self.sender).await.unwrap()
    }
}
