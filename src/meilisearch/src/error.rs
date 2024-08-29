use chrono::DateTime;
use chrono::Utc;
use kyogre_core::DateRangeError;
use kyogre_core::Error as CoreError;
use meilisearch_sdk::tasks::Task;
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [CoreError::Unexpected])]
pub enum Error {
    #[snafu(display("Meilisearch error"))]
    Meilisearch {
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
