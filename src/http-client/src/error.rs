use reqwest::StatusCode;
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("HTTP reqwest error"))]
    #[stack_error(opaque_std = [reqwest::Error, reqwest_middleware::Error])]
    Other {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
    #[snafu(display("HTTP request failed, status: '{status}', url: '{url}', body: '{body}'"))]
    FailedRequest {
        #[snafu(implicit)]
        location: Location,
        url: String,
        status: StatusCode,
        body: String,
    },
}

impl Error {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Error::Other { .. } => None,
            Error::FailedRequest { status, .. } => Some(*status),
        }
    }

    pub fn body(&self) -> Option<&str> {
        match self {
            Error::Other { .. } => None,
            Error::FailedRequest { body, .. } => Some(body),
        }
    }
}
