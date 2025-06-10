use crate::refresher::RefreshRequest;
use kyogre_core::{IsTimeout, MatrixIndexError};
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};
use tokio::sync::mpsc::error::SendError;

pub type Result<T> = std::result::Result<T, Error>;

impl IsTimeout for Error {
    fn is_timeout(&self) -> bool {
        matches!(self, Error::Timeout { .. })
    }
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Failed a duckdb operation"))]
    #[stack_error(skip_from_impls)]
    Duckdb {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: duckdb::Error,
    },
    #[snafu(display("IO error"))]
    #[stack_error(skip_from_impls)]
    Io {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: std::io::Error,
    },
    #[snafu(display("Duckdb timeout error"))]
    #[stack_error(opaque_std = [r2d2::Error])]
    Timeout {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
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
    #[stack_error(skip_from_impls)]
    Grpc {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: Box<tonic::Status>,
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
    #[snafu(display("Matrix index error"))]
    MatrixIndex {
        #[snafu(implicit)]
        location: Location,
        source: MatrixIndexError,
    },
}

impl From<std::io::Error> for Error {
    #[track_caller]
    fn from(value: std::io::Error) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match value.kind() {
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::AddrInUse
            | std::io::ErrorKind::AddrNotAvailable
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::WriteZero
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::Unsupported
            | std::io::ErrorKind::UnexpectedEof
            | std::io::ErrorKind::OutOfMemory
            | std::io::ErrorKind::Other => Error::Timeout {
                location,
                opaque: OpaqueError::Std(Box::new(value)),
            },
            _ => Error::Io {
                location,
                error: value,
            },
        }
    }
}

impl From<tonic::Status> for Error {
    #[track_caller]
    fn from(value: tonic::Status) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match value.code() {
            tonic::Code::Ok
            | tonic::Code::Unknown
            | tonic::Code::InvalidArgument
            | tonic::Code::NotFound
            | tonic::Code::AlreadyExists
            | tonic::Code::PermissionDenied
            | tonic::Code::OutOfRange
            | tonic::Code::Unimplemented
            | tonic::Code::Internal
            | tonic::Code::DataLoss
            | tonic::Code::Unauthenticated => Error::Grpc {
                location,
                error: Box::new(value),
            },
            tonic::Code::FailedPrecondition
            | tonic::Code::ResourceExhausted
            | tonic::Code::Cancelled
            | tonic::Code::DeadlineExceeded
            | tonic::Code::Aborted
            | tonic::Code::Unavailable => Error::Timeout {
                location,
                opaque: OpaqueError::Std(Box::new(value)),
            },
        }
    }
}

impl From<duckdb::Error> for Error {
    #[track_caller]
    fn from(value: duckdb::Error) -> Self {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match &value {
            duckdb::Error::DuckDBFailure(ffi_error, extended_error_message) => {
                match ffi_error.code {
                    duckdb::ErrorCode::InternalMalfunction
                    | duckdb::ErrorCode::DatabaseLocked
                    | duckdb::ErrorCode::OperationAborted
                    | duckdb::ErrorCode::DatabaseBusy
                    | duckdb::ErrorCode::FileLockingProtocolFailed
                    | duckdb::ErrorCode::SchemaChanged
                    | duckdb::ErrorCode::CannotOpen
                    | duckdb::ErrorCode::SystemIoFailure
                    | duckdb::ErrorCode::OutOfMemory => Error::Timeout {
                        location,
                        opaque: OpaqueError::Std(Box::new(value)),
                    },
                    // These error cases have been directly extracted from errors occuring in dev.
                    // They seem to indicate that the failure is transient and we therfore mark
                    // them as a timeout.
                    duckdb::ErrorCode::Unknown => match extended_error_message {
                        Some(msg)
                            if msg.contains("EOF detected")
                                || msg.contains("Connection timed out")
                                || msg.contains("SSL connection has been closed unexpectedly")
                                || msg.contains("server closed the connection unexpectedly") =>
                        {
                            Error::Timeout {
                                location,
                                opaque: OpaqueError::Std(Box::new(value)),
                            }
                        }
                        _ => Error::Duckdb {
                            location,
                            error: value,
                        },
                    },
                    _ => Error::Duckdb {
                        location,
                        error: value,
                    },
                }
            }
            _ => Error::Duckdb {
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
            Error::Timeout { location, .. } => kyogre_core::Error::Timeout {
                location,
                opaque: OpaqueError::Stack(Box::new(value)),
            },
            _ => kyogre_core::Error::Unexpected {
                location,
                opaque: OpaqueError::Std(Box::new(value)),
            },
        }
    }
}
