use std::time::Duration;

use crate::error::{AisMessageError, AisMessageProcessingError, ConsumerError};
use ais_core::{AisPosition, AisStatic, DataMessage};
use error_stack::{bail, IntoReport, Result, ResultExt};
use futures::StreamExt;
use serde::Deserialize;
use tokio::io::AsyncRead;
use tokio::sync::broadcast::Sender;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tracing::{event, instrument, Level};

pub struct Consumer {}

/// The AIS message types we support.
enum SupportedMessageTypes {
    /// A message containing position data.
    Position,
    /// A message containing vessel related data.
    Static,
}

/// Convenience struct to deserialize the message type prior to attempting to deserialize the full
/// message.
#[derive(Deserialize)]
struct MessageType {
    /// What type of message this is.
    #[serde(rename = "messageType")]
    message_type: u32,
}

enum AisMessage {
    Static(AisStatic),
    Position(AisPosition),
}

impl TryFrom<u32> for SupportedMessageTypes {
    type Error = AisMessageError;

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            1 | 2 | 3 | 27 => Ok(SupportedMessageTypes::Position),
            5 | 18 | 19 | 24 => Ok(SupportedMessageTypes::Static),
            _ => Err(AisMessageError::InvalidMessageType(value)),
        }
    }
}

impl Consumer {
    pub async fn run(
        self,
        source: impl AsyncRead + Unpin,
        sender: Sender<DataMessage>,
    ) -> Result<(), ConsumerError> {
        let codec = LinesCodec::new_with_max_length(1000);
        let mut framed_read = FramedRead::new(source, codec);

        let mut buffer = Vec::new();

        let mut interval = tokio::time::interval(Duration::from_secs(10));

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
                    AisMessage::Static(m) => data_message.static_messages.push(m),
                    AisMessage::Position(m) => data_message.positions.push(m),
                },
            },
        }
    }

    // Can only fail if the channel is closed.
    sender
        .send(data_message)
        .into_report()
        .change_context(ConsumerError::InternalChannelClosed)?;

    tracing::Span::current().record("app.num_messages", num_messages);

    Ok(())
}

fn parse_message(message: String) -> Result<AisMessage, AisMessageProcessingError> {
    let message_type: Result<MessageType, AisMessageProcessingError> =
        serde_json::from_str(&message)
            .into_report()
            .change_context(AisMessageProcessingError);

    // The AIS streaming api has a weird behaviour where the `message_type` is sometimes not
    // provided. All observed cases of this indicates that if the message does not contain a
    // `message_type` the message is a [SupportedMessageTypes::Position], but its also possible
    // for a position message to contain a `message_type`.
    if let Ok(message_type) = message_type {
        let supported = SupportedMessageTypes::try_from(message_type.message_type)
            .into_report()
            .change_context(AisMessageProcessingError)?;

        match supported {
            SupportedMessageTypes::Position => {
                let val: AisPosition = serde_json::from_str(&message)
                    .into_report()
                    .change_context(AisMessageProcessingError)?;

                Ok(AisMessage::Position(val))
            }
            SupportedMessageTypes::Static => {
                let val: AisStatic = serde_json::from_str(&message)
                    .into_report()
                    .change_context(AisMessageProcessingError)?;
                Ok(AisMessage::Static(val))
            }
        }
    } else {
        let val: AisPosition = serde_json::from_str(&message)
            .into_report()
            .change_context(AisMessageProcessingError)?;

        Ok(AisMessage::Position(val))
    }
}
