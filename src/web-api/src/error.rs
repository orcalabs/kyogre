use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, ToSchema)]
pub enum ApiError {
    InvalidCallSign,
    InvalidDateRange,
    InternalServerError,
    MissingMmsiOrCallSign,
    Forbidden,
    MissingBwToken,
    InvalidBwToken,
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
            ApiError::MissingMmsiOrCallSign => {
                f.write_str("either mmsi, call sign, or both must be provided")
            }
            ApiError::Forbidden => f.write_str("forbidden"),
            ApiError::MissingBwToken => f.write_str("barentswatch token must be provided"),
            ApiError::InvalidBwToken => f.write_str("an invalid barentswatch token was provided"),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidDateRange
            | ApiError::InvalidCallSign
            | ApiError::MissingMmsiOrCallSign => StatusCode::BAD_REQUEST,
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::MissingBwToken | ApiError::InvalidBwToken => StatusCode::UNAUTHORIZED,
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
