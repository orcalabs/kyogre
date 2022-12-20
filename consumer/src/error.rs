use error_stack::Context;

#[derive(Debug)]
pub enum ConsumerError {
    StreamClosed,
    InternalChannelClosed,
}

impl Context for ConsumerError {}

impl std::fmt::Display for ConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsumerError::StreamClosed => f.write_str("ais stream closed unexpectedly"),
            ConsumerError::InternalChannelClosed => {
                f.write_str("internal broadcast channel closed unexpectedly")
            }
        }
    }
}

#[derive(Debug)]
pub struct AisMessageProcessingError;

impl Context for AisMessageProcessingError {}

impl std::fmt::Display for AisMessageProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("error occured during ais message processing")
    }
}

#[derive(Debug)]
pub enum AisMessageError {
    InvalidMessageType(u32),
}

impl std::error::Error for AisMessageError {}

impl std::fmt::Display for AisMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AisMessageError::InvalidMessageType(message_type) => f.write_fmt(format_args!(
                "encountered an unsupported message type: {}",
                message_type
            )),
        }
    }
}
