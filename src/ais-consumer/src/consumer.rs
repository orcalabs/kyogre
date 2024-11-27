use crate::{
    error::{error::StreamClosedSnafu, Result},
    models::{AisMessage, AisMessageType, AisPosition, AisStatic, MessageType},
};
use async_channel::Sender;
use futures::StreamExt;
use kyogre_core::{DataMessage, NewAisStatic};
use tokio::io::AsyncRead;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tracing::{error, instrument};

pub struct Consumer {
    commit_interval: std::time::Duration,
}

impl Consumer {
    pub fn new(commit_interval: std::time::Duration) -> Consumer {
        Consumer { commit_interval }
    }
    pub async fn run(
        &self,
        source: impl AsyncRead + Unpin,
        sender: Sender<DataMessage>,
    ) -> Result<()> {
        let codec = LinesCodec::new_with_max_length(1000);
        let mut framed_read = FramedRead::new(source, codec);

        // This vector is never deallocated and will match the size of
        // highest amount of messages received during a commit interval.
        let mut buffer = Vec::new();

        let mut interval = tokio::time::interval(self.commit_interval);

        loop {
            tokio::select! {
                message = framed_read.next() => {
                    match message {
                        Some(message) => buffer.push(message),
                        None => {
                            process_messages(buffer.drain(..), &sender).await?;
                            return StreamClosedSnafu{}.fail();
                        },
                    }
                }
                _ = interval.tick() => {
                    process_messages(buffer.drain(..), &sender).await?;
                }
            }
        }
    }
}

// Only returns an error if the receiver half of the sender closes
#[instrument(skip(messages, sender), fields(app.num_messages))]
async fn process_messages<T>(messages: T, sender: &Sender<DataMessage>) -> Result<()>
where
    T: IntoIterator<Item = std::result::Result<String, LinesCodecError>>,
{
    let mut data_message = DataMessage::default();
    let mut num_messages = 0;
    for message in messages {
        num_messages += 1;
        match message {
            Err(e) => error!("failed to consume ais message: {e:?}"),
            Ok(message) => match parse_message(message) {
                Err(e) => error!("failed to parse message: {e:?}"),
                Ok(message) => match message {
                    AisMessage::Static(m) => match NewAisStatic::try_from(m) {
                        Err(e) => error!("failed to convert static message: {e:?}"),
                        Ok(d) => data_message.static_messages.push(d),
                    },
                    AisMessage::Position(m) => {
                        if let Some(m) = m.into() {
                            data_message.positions.push(m)
                        }
                    }
                },
            },
        }
    }

    if data_message.is_empty() {
        return Ok(());
    }

    // Can only fail if the channel is closed.
    sender.send(data_message).await?;

    tracing::Span::current().record("app.num_messages", num_messages);

    Ok(())
}

fn parse_message(message: String) -> Result<AisMessage> {
    let message_type: MessageType = serde_json::from_str(&message)?;

    match message_type.message_type {
        AisMessageType::Position => {
            let val: AisPosition = serde_json::from_str(&message)?;
            Ok(AisMessage::Position(val))
        }
        AisMessageType::Static => {
            let val: AisStatic = serde_json::from_str(&message)?;
            Ok(AisMessage::Static(val))
        }
    }
}
