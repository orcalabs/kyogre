use fiskeridir_rs::{LandingIdError, ParseStringError};
use kyogre_core::{
    ActiveVesselConflict, CatchLocationIdError, ChronoError, DateRangeError, IsTimeout,
    MatrixIndexError,
};
use snafu::{Location, Snafu};
use sqlx::migrate::MigrateError;
use stack_error::{OpaqueError, StackError};

pub(crate) type Result<T> = std::result::Result<T, Error>;

impl IsTimeout for Error {
    fn is_timeout(&self) -> bool {
        matches!(self, Error::Timeout { .. })
    }
}

#[derive(Snafu, StackError)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    #[snafu(display("Json error"))]
    Json {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: serde_json::Error,
    },
    #[snafu(display("Sqlx error"))]
    #[stack_error(skip_from_impls)]
    Sqlx {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: sqlx::Error,
    },
    #[snafu(display("Migrate error"))]
    Migrate {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: MigrateError,
    },
    #[snafu(display("An operation timed out"))]
    #[stack_error(skip_from_impls)]
    Timeout {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: sqlx::Error,
    },
    #[snafu(display("Value unexpectedly missing"))]
    MissingValue {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("could not map inserted trip to back to its corresponding trip positions"))]
    TripPositionMatch {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Found errors when verifying database"))]
    VerifyDatabase {
        #[snafu(implicit)]
        location: Location,
        source: VerifyDatabaseError,
    },
    // Jurisdiction uses `anyhow::Error` as its error type and as that type does not implement
    // `std::error::Error` trait we cant use our regular stack error scheme, instead we stringify the output error.
    #[snafu(display("Jurisdiction error: '{error_stringified}', data: '{data}'"))]
    Jurisdiction {
        #[snafu(implicit)]
        location: Location,
        error_stringified: String,
        data: String,
    },
    #[snafu(display("Data conversion error"))]
    #[stack_error(
        opaque_stack = [
            ParseStringError,
            CatchLocationIdError,
            DateRangeError,
            LandingIdError,
            ChronoError,
            fiskeridir_rs::Error,
            WktConversionError,
            MatrixIndexError
        ],
        opaque_std = [strum::ParseError])]
    Conversion {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
    #[snafu(display("An unexpected error occured"))]
    #[stack_error(opaque_std = [tokio::task::JoinError])]
    Unexpected {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}

// The 'wkt::conversion::Error' contains a 'Box<dyn Error>', note the lack of
// Send/Sync/'static bounds. We need those bounds to use the error directly in our stack-error
// scheme. This type serves as an intermediate error type.
#[derive(Snafu, StackError)]
#[snafu(visibility(pub(crate)))]
pub enum WktConversionError {
    #[snafu(display("Failed WKT conversion, error: '{stringified_error}'"))]
    Convert {
        #[snafu(implicit)]
        location: Location,
        stringified_error: String,
    },
}

#[derive(Snafu, StackError)]
#[snafu(visibility(pub(crate)))]
pub enum VerifyDatabaseError {
    #[snafu(display("Found '{num}' dangling vessel events"))]
    DanglingVesselEvents {
        #[snafu(implicit)]
        location: Location,
        num: i64,
    },
    #[snafu(display("Found hauls with incorrect catch data: '{message_ids:?}'"))]
    IncorrectHaulCatches {
        #[snafu(implicit)]
        location: Location,
        message_ids: Vec<i64>,
    },
    #[snafu(display("Hauls matrix and ers dca living weight differ by '{weight_diff}'"))]
    IncorrectHaulsMatrixLivingWeight {
        #[snafu(implicit)]
        location: Location,
        weight_diff: i64,
    },
    #[snafu(display("Landing matrix and landings living weight differ by '{weight_diff}'"))]
    IncorrectLandingMatrixLivingWeight {
        #[snafu(implicit)]
        location: Location,
        weight_diff: i64,
    },
    #[snafu(display("Vessel conflicts: '{conflicts:#?}'"))]
    ConflictingVesselMappings {
        #[snafu(implicit)]
        location: Location,
        conflicts: Vec<ActiveVesselConflict>,
    },
    #[snafu(display("Landings without trip: '{num}'"))]
    LandingsWithoutTrip {
        #[snafu(implicit)]
        location: Location,
        num: i64,
    },
}

impl From<sqlx::Error> for Error {
    #[track_caller]
    fn from(error: sqlx::Error) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match error {
            sqlx::Error::Database(ref e) => {
                // Postgres error codes documentation:
                // https://www.postgresql.org/docs/current/errcodes-appendix.html
                match e.code().map(|v| v.to_string()).as_deref() {
                    // Deadlock
                    Some("40P01") => Error::Timeout { location, error },
                    _ => Error::Sqlx { location, error },
                }
            }
            sqlx::Error::PoolTimedOut => Error::Timeout { location, error },
            sqlx::Error::Io(ref e) if e.is_timeout() => Error::Timeout { location, error },
            _ => Error::Sqlx { location, error },
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
            Error::Conversion { .. }
            | Error::MissingValue { .. }
            | Error::Json { .. }
            | Error::TripPositionMatch { .. }
            | Error::Sqlx { .. }
            | Error::VerifyDatabase { .. }
            | Error::Jurisdiction { .. }
            | Error::Unexpected { .. }
            | Error::Migrate { .. } => kyogre_core::Error::Unexpected {
                location,
                opaque: OpaqueError::Stack(Box::new(value)),
            },
        }
    }
}
