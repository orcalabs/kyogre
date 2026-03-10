use async_channel::{RecvError, SendError};
use kyogre_core::{DataMessage, IsTimeout, OauthError};
use reqwest::StatusCode;
use snafu::{Location, Snafu};
use stack_error::StackError;
use std::num::ParseIntError;

pub type Result<T> = std::result::Result<T, Error>;

impl IsTimeout for Error {
    fn is_timeout(&self) -> bool {
        match self {
            Error::Http { location: _, error } => error.is_timeout(),
            Error::Oauth {
                location: _,
                source: _,
            } => false,
            Error::FailedRequest {
                location: _,
                url: _,
                status: _,
                body: _,
            } => false,
            Error::StreamClosed { location: _ } => true,
            Error::InternalChannelClosed {
                location: _,
                error: _,
            } => false,
            Error::SendError {
                location: _,
                error: _,
            } => false,
            Error::Io { location: _, error } => error.is_timeout(),
        }
    }
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum ParseMessageError {
    #[snafu(display("Json error"))]
    Json {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: serde_json::Error,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Io error"))]
    Io {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: std::io::Error,
    },
    #[snafu(display("Http error"))]
    Http {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: reqwest::Error,
    },
    #[snafu(display("Oauth error"))]
    Oauth {
        #[snafu(implicit)]
        location: Location,
        source: OauthError,
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
