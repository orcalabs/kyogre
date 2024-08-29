use reqwest::StatusCode;
use snafu::{location, Location, Snafu};
use stack_error::StackError;
use std::num::ParseIntError;

use crate::string_new_types::NonEmptyString;

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
    #[snafu(display("Encountered value that did not match enum an enum variant '{value}'"))]
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

    #[snafu(display("Url parse error"))]
    Url {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: url::ParseError,
    },
    #[snafu(display("Zip error"))]
    Zip {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: zip::result::ZipError,
    },
    // Jurisdiction uses `anyhow::Error` as its error type and as that type does not implement
    // `std::error::Error` trait we cant use our regular stack error scheme, instead we stringify the output error.
    #[snafu(display("Jurisdiction error, error: '{error_stringified}', nation: '{nation:?}', nation_code: '{nation_code:?}'"))]
    Jurisdiction {
        #[snafu(implicit)]
        location: Location,
        error_stringified: String,
        nation_code: Option<NonEmptyString>,
        nation: Option<NonEmptyString>,
    },
    #[snafu(display("Failed data conversion"))]
    Conversion {
        #[snafu(implicit)]
        location: Location,
        source: ParseStringError,
    },
}

impl From<csv::Error> for Error {
    fn from(e: csv::Error) -> Self {
        match e.kind() {
            csv::ErrorKind::Deserialize { pos: _, err } => match err.kind() {
                csv::DeserializeErrorKind::UnexpectedEndOfRow => Error::IncompleteData {
                    error: err.clone(),
                    location: location!(),
                },
                _ => Error::Csv {
                    location: location!(),
                    error: e,
                },
            },
            _ => Error::Csv {
                location: location!(),
                error: e,
            },
        }
    }
}
