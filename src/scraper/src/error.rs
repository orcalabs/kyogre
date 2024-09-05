use std::num::ParseIntError;

use fiskeridir_rs::ParseStringError;
use geozero::error::GeozeroError;
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Failed a database operation"))]
    Database {
        #[snafu(implicit)]
        location: Location,
        source: kyogre_core::Error,
    },
    #[snafu(display("Fiskeridir erorr"))]
    Fiskeridir {
        #[snafu(implicit)]
        location: Location,
        source: fiskeridir_rs::Error,
    },
    #[snafu(display("Task join error"))]
    TaskJoin {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: tokio::task::JoinError,
    },
    #[snafu(display("Csv error"))]
    Csv {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: csv::Error,
    },
    #[snafu(display("Python error"))]
    Python {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: pyo3::PyErr,
    },
    #[snafu(display("Oauth error"))]
    Oauth {
        #[snafu(implicit)]
        location: Location,
        source: kyogre_core::OauthError,
    },
    #[snafu(display("Http error"))]
    Http {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: http_client::Error,
    },
    #[snafu(display("Value unexpectedly missing"))]
    MissingValue {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Regex error"))]
    Regex {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: regex::Error,
    },
    #[snafu(display("Data conversion error"))]
    #[stack_error(
        opaque_stack = [
            ParseStringError,
            TimestampError,
        ],
        opaque_std = [ParseIntError, GeozeroError])]
    Conversion {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum TimestampError {
    #[snafu(display("Malformed filename: '{file_name}'"))]
    Malformed {
        #[snafu(implicit)]
        location: Location,
        file_name: String,
    },
    #[snafu(display("Found an invalid filename: '{file_name}'"))]
    InvalidFilename {
        #[snafu(implicit)]
        location: Location,
        file_name: String,
        #[snafu(source)]
        error: ParseIntError,
    },
    #[snafu(display("Filename contained invalid y/m/d; y: {year}, m: {month}, d: {day}"))]
    InvalidYMD {
        #[snafu(implicit)]
        location: Location,
        year: i32,
        month: u32,
        day: u32,
    },
    #[snafu(display("Filename contained invalid hour: {hour}"))]
    InvalidHour {
        #[snafu(implicit)]
        location: Location,
        hour: u32,
    },
}
