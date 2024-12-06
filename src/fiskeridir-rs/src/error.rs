use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};
use std::num::ParseIntError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum LandingIdError {
    #[snafu(display("LandingId string was invalid length '{value}'"))]
    Length {
        #[snafu(implicit)]
        location: Location,
        value: String,
    },
    #[snafu(display("Failed to parse part of LandingId '{value}'"))]
    Parse {
        #[snafu(implicit)]
        location: Location,
        value: String,
        #[snafu(source)]
        error: ParseIntError,
    },
    #[snafu(display("Encountered value that did not match an enum variant '{value}'"))]
    Invalid {
        #[snafu(implicit)]
        location: Location,
        value: String,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum ParseStringError {
    #[snafu(display("String was unexpectedly empty"))]
    Empty {
        #[snafu(implicit)]
        location: Location,
    },
}

#[derive(Snafu, StackError, strum::EnumDiscriminants)]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Http error"))]
    #[stack_error(opaque_stack = [http_client::Error])]
    Http {
        #[snafu(implicit)]
        location: Location,
        // This is not an `error: http_client::Error` because the compiler warns that the error becomes
        // too large (100+ bytes)
        opaque: OpaqueError,
    },
    #[snafu(display("IO error"))]
    Io {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: std::io::Error,
    },
    #[snafu(display("CSV error"))]
    #[stack_error(skip_from_impls)]
    Csv {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: csv::Error,
    },
    #[snafu(display("Received incomplete csv data"))]
    #[stack_error(skip_from_impls)]
    IncompleteData {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: csv::DeserializeError,
    },
    #[snafu(display("Zip error"))]
    Zip {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: zip::result::ZipError,
    },
    #[snafu(display("Parse int error"))]
    ParseInt {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: ParseIntError,
    },
}

impl From<csv::Error> for Error {
    #[track_caller]
    fn from(e: csv::Error) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match e.kind() {
            csv::ErrorKind::Deserialize { pos: _, err } => match err.kind() {
                csv::DeserializeErrorKind::UnexpectedEndOfRow => Error::IncompleteData {
                    error: err.clone(),
                    location,
                },
                _ => Error::Csv { location, error: e },
            },
            _ => Error::Csv { location, error: e },
        }
    }
}
