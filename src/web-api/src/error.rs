use actix_web::{
    body::BoxBody,
    http::{header::ToStrError, StatusCode},
    HttpResponse, ResponseError,
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::ParseStringError;
use kyogre_core::DateRangeError;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};
use utoipa::ToSchema;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum BwError {
    #[snafu(display(
        "Failed to lookup barentswatch profile status: '{status}', url: '{url}', body: '{body}'"
    ))]
    Profile {
        #[snafu(implicit)]
        location: Location,
        url: String,
        status: reqwest::StatusCode,
        body: String,
    },
}

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum JWTDecodeError {
    #[snafu(display("tried to decode a token with at disabled Auth0Guard"))]
    Disabled {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("A JWK value was missing"))]
    MissingValue {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The provided JWT token could no be decoded"))]
    Decode {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: jsonwebtoken::errors::Error,
    },
}

#[derive(Snafu, StackError, strum::EnumDiscriminants)]
#[strum_discriminants(derive(Deserialize, Serialize, ToSchema))]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display("Start date: '{start}' cannot be after end date: '{end}'"))]
    StartAfterEnd {
        #[snafu(implicit)]
        location: Location,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    #[snafu(display("The given call sign '{call_sign}' was invalid"))]
    InvalidCallSign {
        #[snafu(implicit)]
        location: Location,
        source: ParseStringError,
        call_sign: String,
    },
    #[snafu(display("Missing barentswatch fisk info profile"))]
    MissingBwFiskInfoProfile {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The given start/end pair was invalid, start: '{start}' end: '{end}'"))]
    InvalidDateRange {
        #[snafu(implicit)]
        location: Location,
        source: DateRangeError,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    #[snafu(display("Either both start and end must be specified or neither, start_given: '{start}', end_given: '{end}'"))]
    MissingDateRange {
        #[snafu(implicit)]
        location: Location,
        start: bool,
        end: bool,
    },
    #[snafu(display("Either trip_id, mmsi, or call sign must be provided"))]
    MissingMmsiOrCallSignOrTripId {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Insufficient permissions for requested operation"))]
    InsufficientPermissions {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("JWT token is missing"))]
    MissingJWT {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("An invalid JWT token was provided"))]
    InvalidJWT {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The provided JWT token could no be parsed"))]
    ParseJWT {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: ToStrError,
    },
    #[snafu(display("The provided JWT token could not be decoded"))]
    JWTDecode {
        #[snafu(implicit)]
        location: Location,
        source: JWTDecodeError,
    },
    #[snafu(display("An unexpected error occured"))]
    #[stack_error(
        opaque_stack = [kyogre_core::Error, BwError],
        opaque_std = [serde_json::Error, reqwest::Error])]
    Unexpected {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorDiscriminants,
    pub description: String,
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        use ErrorDiscriminants::*;
        let disc: ErrorDiscriminants = self.into();
        match disc {
            StartAfterEnd
            | InvalidCallSign
            | MissingBwFiskInfoProfile
            | InvalidDateRange
            | MissingDateRange
            | MissingMmsiOrCallSignOrTripId => StatusCode::BAD_REQUEST,
            InsufficientPermissions => StatusCode::FORBIDDEN,
            MissingJWT | InvalidJWT | ParseJWT | JWTDecode => StatusCode::UNAUTHORIZED,
            Unexpected => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let error = ErrorResponse {
            error: self.into(),
            description: format!("{self}"),
        };
        HttpResponse::build(self.status_code()).json(&error)
    }
}
