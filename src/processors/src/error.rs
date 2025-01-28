use kyogre_core::Error as CoreError;
use snafu::{Location, Snafu};
use stack_error::StackError;
use tokio::task::JoinError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [CoreError::Unexpected])]
pub enum Error {
    #[snafu(display("Failed to join tasks"))]
    JoinError {
        #[snafu(implicit)]
        location: Location,
        error: JoinError,
    },
    #[snafu(display("Failed a database operation"))]
    Database {
        #[snafu(implicit)]
        location: Location,
        source: kyogre_core::Error,
    },
    #[snafu(display(
        "Failed to estimate distance between locations, from '{from:?}', to '{to:?}', error '{error_stringified}'"
    ))]
    DistanceEstimation {
        #[snafu(implicit)]
        location: Location,
        error_stringified: String,
        from: geoutils::Location,
        to: geoutils::Location,
    },
    #[snafu(display("Invalid date range"))]
    InvalidDateRange {
        #[snafu(implicit)]
        location: Location,
        source: kyogre_core::DateRangeError,
    },
}
