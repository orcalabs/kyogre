use actix_web::{
    HttpResponse, ResponseError,
    body::BoxBody,
    error::QueryPayloadError,
    http::{StatusCode, header::ToStrError},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, OrgId, ParseStringError};
use kyogre_core::{DateRangeError, WebApiError};
use oasgen::OaSchema;
use serde::{Deserialize, Serialize};
use snafu::{Location, Snafu};
use stack_error::{OpaqueError, StackError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Snafu, StackError)]
#[snafu(module, visibility(pub))]
pub enum JWTDecodeError {
    #[snafu(display("Tried to decode a token with a disabled auth config"))]
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
#[strum_discriminants(derive(Deserialize, Serialize, OaSchema))]
#[snafu(module, visibility(pub))]
pub enum Error {
    #[snafu(display(
        "Fuel after '{fuel_after_liter}' cannot be lower or equal to fuel '{fuel_liter}'"
    ))]
    FuelAfterLowerThanFuel {
        #[snafu(implicit)]
        location: Location,
        fuel_after_liter: f64,
        fuel_liter: f64,
    },
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
    #[snafu(display(
        "Either both start and end must be specified or neither, start_given: '{start}', end_given: '{end}'"
    ))]
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
    #[snafu(display("The JWT issuer is unknown"))]
    UnknownIssuer {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: strum::ParseError,
    },
    #[snafu(display("JWT token is missing"))]
    MissingJWT {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("An invalid JWT token was provided"))]
    #[stack_error(skip_from_impls)]
    InvalidJWT {
        #[snafu(implicit)]
        location: Location,
        source: http_client::Error,
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
    #[snafu(display("The provided JWT token was invalid"))]
    InvalidJWTParts {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("The provided base64 data could not be decoded"))]
    Base64Decode {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        error: base64::DecodeError,
    },
    #[snafu(display("An invalid Excel document was provided"))]
    #[stack_error(opaque_std = [calamine::DeError, calamine::XlsxError])]
    InvalidExcel {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
    #[snafu(display("Query payload error"))]
    QueryPayload {
        #[snafu(implicit)]
        location: Location,
        error: QueryPayloadError,
    },
    #[snafu(display("The vessel with call_sign '{call_sign}' was not found"))]
    UpdateVesselNotFound {
        #[snafu(implicit)]
        location: Location,
        call_sign: CallSign,
    },
    #[snafu(display("The org '{org_id}' was not found"))]
    OrgNotFound {
        #[snafu(implicit)]
        location: Location,
        org_id: OrgId,
    },
    #[snafu(display("The callsign '{call_sign}' does not exist"))]
    CallSignDoesNotExist {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
        call_sign: CallSign,
    },
    #[snafu(display("An unexpected error occured"))]
    #[stack_error(
        opaque_stack = [kyogre_core::Error, http_client::Error, ParseStringError],
        opaque_std = [serde_json::Error])]
    Unexpected {
        #[snafu(implicit)]
        location: Location,
        opaque: OpaqueError,
    },
}

#[derive(Serialize, Deserialize, OaSchema)]
pub struct ErrorResponse {
    pub error: ErrorDiscriminants,
    pub description: String,
}

impl ResponseError for Error {
    fn status_code(&self) -> StatusCode {
        use ErrorDiscriminants::*;
        match ErrorDiscriminants::from(self) {
            StartAfterEnd
            | InvalidCallSign
            | MissingBwFiskInfoProfile
            | InvalidDateRange
            | MissingDateRange
            | QueryPayload
            | FuelAfterLowerThanFuel
            | Base64Decode
            | InvalidExcel
            | CallSignDoesNotExist
            | MissingMmsiOrCallSignOrTripId => StatusCode::BAD_REQUEST,
            InsufficientPermissions => StatusCode::FORBIDDEN,
            MissingJWT | InvalidJWT | ParseJWT | JWTDecode | UnknownIssuer | InvalidJWTParts => {
                StatusCode::UNAUTHORIZED
            }
            UpdateVesselNotFound | OrgNotFound => StatusCode::NOT_FOUND,
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

impl From<WebApiError> for Error {
    #[track_caller]
    fn from(val: WebApiError) -> Error {
        let location = std::panic::Location::caller();
        let location = Location::new(location.file(), location.line(), location.column());
        match val {
            WebApiError::CallSignDoesNotExist {
                call_sign, opaque, ..
            } => Error::CallSignDoesNotExist {
                location,
                opaque,
                call_sign,
            },
            WebApiError::Timeout { opaque, .. } | WebApiError::Unexpected { opaque, .. } => {
                Error::Unexpected { location, opaque }
            }
        }
    }
}
