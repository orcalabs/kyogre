use kyogre_core::DateRangeError;
use kyogre_core::Error as CoreError;
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};

use crate::ErsEvent;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [CoreError::Unexpected])]
pub enum Error {
    #[snafu(display("Python error"))]
    Python {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: pyo3::PyErr,
    },
    #[snafu(display("Json error"))]
    Json {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: serde_json::Error,
    },
    #[snafu(display("Database operation failed"))]
    Database {
        #[snafu(implicit)]
        location: Location,
        source: kyogre_core::Error,
    },
    #[snafu(display("An ers based trip started on an arrival '{event:?}'"))]
    TripStartedOnArrival {
        #[snafu(implicit)]
        location: Location,
        event: ErsEvent,
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
    #[snafu(display("Data conversion error"))]
    #[stack_error(opaque_stack_from = [DateRangeError])]
    Conversion {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}
