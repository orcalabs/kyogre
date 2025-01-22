use crate::IsTimeout;
use chrono::{DateTime, NaiveDate, Utc};
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};
use std::num::ParseIntError;

pub type CoreResult<T> = Result<T, Error>;

impl IsTimeout for Error {
    fn is_timeout(&self) -> bool {
        matches!(self, Error::Timeout { .. })
    }
}

impl IsTimeout for std::io::Error {
    fn is_timeout(&self) -> bool {
        use std::io::ErrorKind;
        matches!(
            self.kind(),
            ErrorKind::ConnectionRefused
                | ErrorKind::ConnectionReset
                | ErrorKind::ConnectionAborted
                | ErrorKind::NotConnected
                | ErrorKind::AddrInUse
                | ErrorKind::AddrNotAvailable
                | ErrorKind::BrokenPipe
                | ErrorKind::WouldBlock
                | ErrorKind::TimedOut
                | ErrorKind::WriteZero
                | ErrorKind::Interrupted
                | ErrorKind::Unsupported
                | ErrorKind::UnexpectedEof
                | ErrorKind::OutOfMemory
                | ErrorKind::Other
        )
    }
}

#[derive(Snafu, StackError)]
#[snafu(module)]
pub enum Error {
    #[snafu(display("Timeout error"))]
    Timeout {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
    #[snafu(display("Unexpected error"))]
    #[stack_error(opaque_std = [tokio::task::JoinError])]
    Unexpected {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [Error::Unexpected])]
pub enum CatchLocationIdError {
    #[snafu(display("Invalid catch location id length '{id}'"))]
    Length {
        #[snafu(implicit)]
        location: Location,
        id: String,
    },
    #[snafu(display("Could not parse area codes '{id}'"))]
    Parse {
        #[snafu(implicit)]
        location: Location,
        id: String,
        #[snafu(source)]
        error: ParseIntError,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [Error::Unexpected])]
pub enum OauthError {
    #[snafu(display("Url parse Error"))]
    Url {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: oauth2::url::ParseError,
    },

    #[snafu(display("Failed to exchange credentials"))]
    ExchangeCredentials {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: Box<dyn std::error::Error + Send + Sync>,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [Error::Unexpected])]
pub enum MatrixIndexError {
    #[snafu(display("Invalid value '{val}'"))]
    Value {
        #[snafu(implicit)]
        location: Location,
        val: i32,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [Error::Unexpected])]
pub enum RangeError {
    #[snafu(display("Range was invalid '{val}'"))]
    Invalid {
        #[snafu(implicit)]
        location: Location,
        val: String,
    },
    #[snafu(display("Range was invalid '{val}'"))]
    ParseBound {
        #[snafu(implicit)]
        location: Location,
        val: String,
        #[snafu(source)]
        error: Box<dyn std::error::Error + Send + Sync>,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [Error::Unexpected])]
pub enum DateRangeError {
    #[snafu(display("start of date range cannot be after end, '{start}', '{end}'"))]
    Ordering {
        #[snafu(implicit)]
        location: Location,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    #[snafu(display("encountered and unexpected unbounded range"))]
    Unbounded {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Invalid calendar date: {date}"))]
    InvalidCalendarDate {
        #[snafu(implicit)]
        location: Location,
        date: NaiveDate,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum BuyerLocationError {
    #[snafu(display("Buyer location contained more than 1 approval numbers: {num}"))]
    TooManyApprovalNumbers {
        #[snafu(implicit)]
        location: Location,
        num: usize,
    },
}

#[cfg(feature = "sqlx")]
mod _sqlx {
    use snafu::{Location, Snafu};
    use stack_error::StackError;

    #[derive(Snafu, StackError)]
    #[snafu(module, visibility(pub))]
    pub enum DecodeError {
        #[snafu(display("Value unexpectedly missing"))]
        MissingValue {
            #[snafu(implicit)]
            location: Location,
        },
    }
}

#[cfg(feature = "sqlx")]
pub use _sqlx::*;
