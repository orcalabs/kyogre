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

#[derive(Debug)]
pub enum BarentswatchClientError {
    RequestCreation,
    SendingRequest,
    Body,
    Server { response_code: u16, body: String },
}

impl Context for BarentswatchClientError {}

impl std::fmt::Display for BarentswatchClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BarentswatchClientError::RequestCreation => f.write_str("failed to construct request"),
            BarentswatchClientError::Body => f.write_str("failed to read the request body"),
            BarentswatchClientError::SendingRequest => {
                f.write_str("failed to send request to server")
            }
            BarentswatchClientError::Server {
                response_code,
                body,
            } => f.write_fmt(format_args!(
                "non-ok response received from server, status_code: {}, body: {}",
                response_code, body
            )),
        }
    }
}

#[derive(Debug)]
pub enum BearerTokenError {
    Configuration,
    Acquisition,
}

impl Context for BearerTokenError {}

impl std::fmt::Display for BearerTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BearerTokenError::Configuration => f.write_str("invalid oauth configuration"),
            BearerTokenError::Acquisition => f.write_str("failed to acquire token"),
        }
    }
}
