use async_channel::{RecvError, SendError};
use kyogre_core::DataMessage;
use reqwest::StatusCode;
use snafu::{Location, Snafu};
use stack_error::StackError;
use std::num::ParseIntError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Json error"))]
    Json {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: serde_json::Error,
    },
    #[snafu(display("Http error"))]
    Http {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: reqwest::Error,
    },
    #[snafu(display("HTTP Request failed, status: '{status}', url: '{url}', body: '{body}'"))]
    FailedRequest {
        #[snafu(implicit)]
        location: Location,
        url: String,
        status: StatusCode,
        body: String,
    },
    #[snafu(display("Consumer stream closed unexpectedly"))]
    StreamClosed {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Internal ais channel closed"))]
    InternalChannelClosed {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: RecvError,
    },
    #[snafu(display("Failed to send message to consume loop"))]
    SendError {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: SendError<DataMessage>,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum AisMessageError {
    #[snafu(display("Encountered an unsupported message type '{message_type}'"))]
    InvalidMessageType {
        #[snafu(implicit)]
        location: Location,
        message_type: u32,
    },
    #[snafu(display("Encountered an unexpected estimated-time-of-arrival value length '{eta}'"))]
    InvalidEta {
        #[snafu(implicit)]
        location: Location,
        eta: String,
    },
    #[snafu(display("Failed to parse part of estimated-time-of-arrival value '{eta}'"))]
    ParseEta {
        #[snafu(implicit)]
        location: Location,
        eta: String,
        #[snafu(source)]
        error: ParseIntError,
    },
}
