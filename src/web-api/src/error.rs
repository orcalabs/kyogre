use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, ToSchema)]
pub enum ApiError {
    StartAfterEnd {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    },
    InvalidCallSign,
    MissingBwFiskInfoProfile,
    InvalidDateRange,
    InternalServerError,
    MissingMmsiOrCallSignOrTripId,
    Forbidden,
    MissingBwToken,
    InvalidBwToken,
    InvalidLandingId,
    InvalidSpeciesGroupId(u32),
    InvalidAuthToken,
    MissingAuthToken,
}

impl std::error::Error for ApiError {}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    error: ApiError,
    description: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::InvalidDateRange => {
                f.write_str("an invalid start/end date pair was received")
            }
            ApiError::InternalServerError => f.write_str("an internal server error occured"),
            ApiError::InvalidCallSign => f.write_str("an invalid call sign was received"),
            ApiError::MissingBwFiskInfoProfile => {
                f.write_str("missing barentswatch fisk info profile")
            }
            ApiError::MissingMmsiOrCallSignOrTripId => {
                f.write_str("either trip_id, mmsi or call sign must be provided")
            }
            ApiError::Forbidden => f.write_str("forbidden"),
            ApiError::MissingBwToken => f.write_str("barentswatch token must be provided"),
            ApiError::InvalidBwToken => f.write_str("an invalid barentswatch token was provided"),
            ApiError::InvalidLandingId => f.write_str("an invalid landing id was provided"),
            ApiError::StartAfterEnd { start, end } => f.write_fmt(format_args!(
                "start date: {start} cannot be after end date: {end}"
            )),
            ApiError::InvalidSpeciesGroupId(val) => {
                f.write_fmt(format_args!("invalid species group id specified: {val}"))
            }
            ApiError::InvalidAuthToken => f.write_str("received an invalid authorization token"),
            ApiError::MissingAuthToken => f.write_str("missing authorization token"),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidDateRange
            | ApiError::InvalidCallSign
            | ApiError::MissingBwFiskInfoProfile
            | ApiError::InvalidLandingId
            | ApiError::StartAfterEnd { start: _, end: _ }
            | ApiError::InvalidSpeciesGroupId(_)
            | ApiError::MissingMmsiOrCallSignOrTripId => StatusCode::BAD_REQUEST,
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::MissingBwToken
            | ApiError::InvalidBwToken
            | ApiError::InvalidAuthToken
            | ApiError::MissingAuthToken => StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let error = ErrorResponse {
            error: *self,
            description: format!("{self}"),
        };
        HttpResponse::build(self.status_code()).json(&error)
    }
}
