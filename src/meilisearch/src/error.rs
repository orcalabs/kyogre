use chrono::DateTime;
use chrono::Utc;
use kyogre_core::DateRangeError;
use kyogre_core::Error as CoreError;
use kyogre_core::IsTimeout;
use meilisearch_sdk::tasks::Task;
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};

pub type Result<T> = std::result::Result<T, Error>;

impl IsTimeout for Error {
    fn is_timeout(&self) -> bool {
        matches!(self, Error::Timeout { .. })
    }
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Meilisearch error"))]
    #[stack_error(skip_from_impls)]
    Meilisearch {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: meilisearch_sdk::errors::Error,
    },
    #[snafu(display("Meilisearch timeout"))]
    #[stack_error(skip_from_impls)]
    Timeout {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: meilisearch_sdk::errors::Error,
    },
    #[snafu(display("Task failed '{task:?}'"))]
    Task {
        #[snafu(implicit)]
        location: Location,
        task: Box<Task>,
    },
    #[snafu(display("Database operation failed"))]
    Database {
        #[snafu(implicit)]
        location: Location,
        source: kyogre_core::Error,
    },
    #[snafu(display("Failed data conversion"))]
    #[stack_error(opaque_stack = [DateRangeError, TimeStampError])]
    Conversion {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [CoreError::Unexpected])]
pub enum TimeStampError {
    #[snafu(display("Could not convert timpestamp to nanos '{ts}'"))]
    Conversion {
        #[snafu(implicit)]
        location: Location,
        ts: DateTime<Utc>,
    },
}

impl From<meilisearch_sdk::errors::Error> for Error {
    #[track_caller]
    fn from(value: meilisearch_sdk::errors::Error) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match value {
            meilisearch_sdk::errors::Error::HttpError(ref err) => match err.status() {
                Some(e) if e.is_server_error() => Error::Timeout {
                    location,
                    error: value,
                },
                _ => Error::Meilisearch {
                    location,
                    error: value,
                },
            },
            meilisearch_sdk::errors::Error::Timeout => Error::Timeout {
                location,
                error: value,
            },
            _ => Error::Meilisearch {
                location,
                error: value,
            },
        }
    }
}

impl From<Error> for kyogre_core::Error {
    #[track_caller]
    fn from(value: Error) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match value {
            Error::Timeout { .. } => kyogre_core::Error::Timeout {
                location,
                opaque: OpaqueError::Stack(Box::new(value)),
            },
            _ => kyogre_core::Error::Unexpected {
                location,
                opaque: OpaqueError::Stack(Box::new(value)),
            },
        }
    }
}
