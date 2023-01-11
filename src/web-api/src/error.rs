use actix_web::{body::BoxBody, http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ApiError {
    InvalidDateRange,
    InternalServerError,
}

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
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InvalidDateRange => StatusCode::BAD_REQUEST,
            ApiError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let error = ErrorResponse {
            error: *self,
            description: format!("{}", self),
        };
        HttpResponse::build(self.status_code()).json(&error)
    }
}
