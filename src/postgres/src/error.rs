use error_stack::{report, Context, Report};
use kyogre_core::{
    CatchLocationIdError, DateRangeError, DeleteError, HaulMatrixIndexError, InsertError,
    LandingMatrixIndexError, QueryError, UpdateError,
};

use crate::models::ActiveVesselConflict;

#[derive(Debug)]
pub enum PostgresError {
    Connection,
    Transaction,
    Query,
    DataConversion,
    InconsistentState,
}

impl Context for PostgresError {}

impl std::fmt::Display for PostgresError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PostgresError::Connection => f.write_str("failed to acquire a database connection"),
            PostgresError::Transaction => f.write_str("failed to start/commit transaction"),
            PostgresError::Query => f.write_str("a query related error occured"),
            PostgresError::DataConversion => {
                f.write_str("failed to convert data to postgres specific data type")
            }
            PostgresError::InconsistentState => {
                f.write_str("database found to be in an inconsistent state")
            }
        }
    }
}

#[derive(Debug)]
pub struct UnboundedRangeError;

impl std::error::Error for UnboundedRangeError {}

impl std::fmt::Display for UnboundedRangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("encountered and unexpected unbounded range")
    }
}

#[derive(Debug)]
pub struct PortCoordinateError(pub String);

impl std::error::Error for PortCoordinateError {}

impl std::fmt::Display for PortCoordinateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "the port_id {} has one of latitude and longitude set, but not both",
            self.0
        ))
    }
}

#[derive(Debug)]
pub enum VerifyDatabaseError {
    DanglingVesselEvents(i64),
    IncorrectHaulCatches(Vec<i64>),
    IncorrectHaulsMatrixLivingWeight(i64),
    IncorrectLandingMatrixLivingWeight(i64),
    ConflictingVesselMappings(Vec<ActiveVesselConflict>),
    LandingsWithoutTrip(i64),
}

impl std::error::Error for VerifyDatabaseError {}

impl std::fmt::Display for VerifyDatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyDatabaseError::DanglingVesselEvents(v) => {
                f.write_fmt(format_args!("found {v} dangling vessel events"))
            }
            VerifyDatabaseError::IncorrectHaulCatches(v) => {
                f.write_fmt(format_args!("found hauls with incorrect catch data: {v:?}"))
            }
            VerifyDatabaseError::IncorrectHaulsMatrixLivingWeight(v) => f.write_fmt(format_args!(
                "hauls matrix and ers dca living weight differ by {v}"
            )),
            VerifyDatabaseError::IncorrectLandingMatrixLivingWeight(v) => f.write_fmt(
                format_args!("landing matrix and landings living weight differ by {v}"),
            ),
            VerifyDatabaseError::ConflictingVesselMappings(v) => {
                f.write_fmt(format_args!("vessel conflicts: {:#?}", v))
            }
            VerifyDatabaseError::LandingsWithoutTrip(v) => {
                f.write_fmt(format_args!("landings without trip: {}", v))
            }
        }
    }
}

#[derive(Debug)]
pub struct PostgresErrorWrapper(Report<PostgresError>);

impl PostgresErrorWrapper {
    pub fn into_inner(self) -> Report<PostgresError> {
        self.0
    }
}

impl From<Report<PostgresError>> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: Report<PostgresError>) -> Self {
        Self(value)
    }
}

impl From<Report<HaulMatrixIndexError>> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: Report<HaulMatrixIndexError>) -> Self {
        Self(value.change_context(PostgresError::DataConversion))
    }
}

impl From<Report<LandingMatrixIndexError>> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: Report<LandingMatrixIndexError>) -> Self {
        Self(value.change_context(PostgresError::DataConversion))
    }
}

impl From<Report<CatchLocationIdError>> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: Report<CatchLocationIdError>) -> Self {
        Self(value.change_context(PostgresError::DataConversion))
    }
}

impl From<Report<fiskeridir_rs::Error>> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: Report<fiskeridir_rs::Error>) -> Self {
        Self(value.change_context(PostgresError::DataConversion))
    }
}

impl From<sqlx::Error> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: sqlx::Error) -> Self {
        Self(report!(PostgresError::Query).attach_printable(format!("{value:?}")))
    }
}

impl From<VerifyDatabaseError> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: VerifyDatabaseError) -> Self {
        Self(report!(value).change_context(PostgresError::InconsistentState))
    }
}

impl From<PortCoordinateError> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: PortCoordinateError) -> Self {
        Self(report!(value).change_context(PostgresError::DataConversion))
    }
}

impl From<serde_json::Error> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: serde_json::Error) -> Self {
        Self(report!(value).change_context(PostgresError::DataConversion))
    }
}

impl From<DateRangeError> for PostgresErrorWrapper {
    #[track_caller]
    fn from(value: DateRangeError) -> Self {
        Self(report!(value).change_context(PostgresError::DataConversion))
    }
}

impl From<PostgresErrorWrapper> for Report<QueryError> {
    fn from(value: PostgresErrorWrapper) -> Self {
        value.0.change_context(QueryError)
    }
}

impl From<PostgresErrorWrapper> for Report<InsertError> {
    fn from(value: PostgresErrorWrapper) -> Self {
        value.0.change_context(InsertError)
    }
}

impl From<PostgresErrorWrapper> for Report<UpdateError> {
    fn from(value: PostgresErrorWrapper) -> Self {
        value.0.change_context(UpdateError)
    }
}

impl From<PostgresErrorWrapper> for Report<DeleteError> {
    fn from(value: PostgresErrorWrapper) -> Self {
        value.0.change_context(DeleteError)
    }
}
