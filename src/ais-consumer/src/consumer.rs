use crate::{
    error::{AisMessageProcessingError, ConsumerError},
    models::{
        AisMessage, AisMessageType, AisPosition, AisStatic, MessageType, NewAisPositionWrapper,
    },
};
use error_stack::{bail, Result, ResultExt};
use futures::StreamExt;
use kyogre_core::{DataMessage, NewAisStatic};
use tokio::io::AsyncRead;
use tokio::sync::broadcast::Sender;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tracing::{event, instrument, Level};

pub struct Consumer {
    commit_interval: std::time::Duration,
}

impl Consumer {
    pub fn new(commit_interval: std::time::Duration) -> Consumer {
        Consumer { commit_interval }
    }
    pub async fn run(
        self,
        source: impl AsyncRead + Unpin,
        sender: Sender<DataMessage>,
    ) -> Result<(), ConsumerError> {
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
                        None => bail!(ConsumerError::StreamClosed),
                    }
                }
                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        process_messages(buffer.drain(..), &sender).await?;
                    }
                }
            }
        }
    }
}

#[instrument(skip(messages, sender), fields(app.num_messages))]
async fn process_messages<T>(messages: T, sender: &Sender<DataMessage>) -> Result<(), ConsumerError>
where
    T: IntoIterator<Item = std::result::Result<String, LinesCodecError>>,
{
    let mut data_message = DataMessage::default();
    let mut num_messages = 0;
    for message in messages {
        num_messages += 1;
        match message {
            Err(e) => event!(Level::ERROR, "failed to consume ais message: {:?}", e),
            Ok(message) => match parse_message(message) {
                Err(e) => event!(Level::ERROR, "{:?}", e),
                Ok(message) => match message {
                    AisMessage::Static(m) => match NewAisStatic::try_from(m) {
                        Err(e) => event!(Level::ERROR, "{:?}", e),
                        Ok(d) => data_message.static_messages.push(d),
                    },
                    AisMessage::Position(m) => {
                        if let Some(m) = NewAisPositionWrapper::from(m).0 {
                            data_message.positions.push(m)
                        }
                    }
                },
            },
        }
    }

    // Can only fail if the channel is closed.
    sender
        .send(data_message)
        .change_context(ConsumerError::InternalChannelClosed)?;

    tracing::Span::current().record("app.num_messages", num_messages);

    Ok(())
}

fn parse_message(message: String) -> Result<AisMessage, AisMessageProcessingError> {
    let message_type: MessageType =
        serde_json::from_str(&message).change_context(AisMessageProcessingError)?;

    match message_type.message_type {
        AisMessageType::Position => {
            let val: AisPosition =
                serde_json::from_str(&message).change_context(AisMessageProcessingError)?;

            Ok(AisMessage::Position(val))
        }
        AisMessageType::Static => {
            let val: AisStatic =
                serde_json::from_str(&message).change_context(AisMessageProcessingError)?;
            Ok(AisMessage::Static(val))
        }
    }
}
