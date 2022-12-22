use crate::{
    error::{AisMessageProcessingError, ConsumerError},
    models::{
        AisMessage, AisPosition, AisStatic, MessageType, NewAisPositionWrapper,
        SupportedMessageTypes,
    },
};
use ais_core::{DataMessage, NewAisStatic};
use error_stack::{bail, IntoReport, Result, ResultExt};
use futures::StreamExt;
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
        cancellation: Option<tokio::sync::mpsc::Receiver<()>>,
    ) -> Result<(), ConsumerError> {
        let codec = LinesCodec::new_with_max_length(1000);
        let mut framed_read = FramedRead::new(source, codec);

        let enable_cancellation = cancellation.is_some();
        let mut cancellation = if let Some(c) = cancellation {
            c
        } else {
            let (_, recv) = tokio::sync::mpsc::channel(1);
            recv
        };

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
                _ = cancellation.recv(), if enable_cancellation => {
                    break Ok(());
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
                    AisMessage::Static(m) => {
                        data_message.static_messages.push(NewAisStatic::from(m))
                    }
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
