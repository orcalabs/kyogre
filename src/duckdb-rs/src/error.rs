use crate::refresher::RefreshRequest;
use kyogre_core::{Error as CoreError, MatrixIndexError};
use snafu::{Location, Snafu};
use stack_error::StackError;
use tokio::sync::mpsc::error::SendError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
#[stack_error(to = [CoreError::Unexpected])]
pub enum Error {
    #[snafu(display("Failed a duckdb operation"))]
    Duckdb {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: duckdb::Error,
    },
    #[snafu(display("IO error"))]
    Io {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: std::io::Error,
    },
    #[snafu(display("Refresh channel send error"))]
    Refresh {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: SendError<RefreshRequest>,
    },
    #[snafu(display("Refresh communication failed"))]
    RefreshCommuniction {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Grpc error"))]
    Grpc {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: tonic::Status,
    },
    #[snafu(display("Uri error"))]
    Uri {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: http::uri::InvalidUri,
    },
    #[snafu(display("Invalid parameters where provided '{value}'"))]
    InvalidParameters {
        #[snafu(implicit)]
        location: Location,
        value: u32,
    },
    #[snafu(display("Connection error"))]
    Connection {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: r2d2::Error,
    },
    #[snafu(display("Matrix index error"))]
    MatrixIndex {
        #[snafu(implicit)]
        location: Location,
        source: MatrixIndexError,
    },
}
