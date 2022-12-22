use crate::{consumer::Consumer, settings::Settings};
use ais_core::DataMessage;
use postgres::PostgresAdapter;
use tokio::{
    io::AsyncRead,
    sync::broadcast::{self, Receiver, Sender},
};

pub struct App {
    consumer: Consumer,
    sender: Sender<DataMessage>,
}

impl App {
    pub async fn build(
        settings: Settings,
        postgres_cancellation: Option<tokio::sync::mpsc::Receiver<()>>,
    ) -> App {
        let (sender, receiver) = broadcast::channel::<DataMessage>(settings.broadcast_buffer_size);
        let adapter = PostgresAdapter::new(&settings.postgres).await.unwrap();

        tokio::spawn(adapter.consume_loop(receiver, postgres_cancellation));

        App {
            sender,
            consumer: Consumer::new(settings.commit_interval),
        }
    }

    pub fn subscribe(&self) -> Receiver<DataMessage> {
        self.sender.subscribe()
    }

    pub async fn run(
        self,
        source: impl AsyncRead + Unpin,
        cancellation: Option<tokio::sync::mpsc::Receiver<()>>,
    ) {
        self.consumer
            .run(source, self.sender, cancellation)
            .await
            .unwrap()
    }
}
