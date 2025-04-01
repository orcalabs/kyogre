use super::BearerToken;
use crate::{
    error::{Error, error::MissingJWTSnafu},
    states::Auth0State,
};
use actix_web::{FromRequest, http::header::AUTHORIZATION, web::Data};
use futures::future::{Ready, ready};
use kyogre_core::AisPermission;
use oasgen::{
    HeaderStyle, OaParameter, OaSchema, Parameter, ParameterData, ParameterKind,
    ParameterSchemaOrContent, RefOr,
};
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, EnumIter)]
pub enum Auth0Permission {
    #[serde(rename = "read:ais:under_15m")]
    ReadAisUnder15m,
    #[serde(rename = "read:fishing_facility")]
    ReadFishingFacility,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Auth0Profile {
    pub sub: String,
    pub exp: i64,
    pub permissions: Vec<Auth0Permission>,
}

impl From<&Auth0Profile> for AisPermission {
    fn from(v: &Auth0Profile) -> Self {
        if v.permissions.contains(&Auth0Permission::ReadAisUnder15m) {
            AisPermission::All
        } else {
            AisPermission::Above15m
        }
    }
}

impl OaParameter for Auth0Profile {
    fn parameters() -> Vec<RefOr<Parameter>> {
        vec![RefOr::Item(Parameter {
            data: ParameterData {
                name: AUTHORIZATION.to_string(),
                description: None,
                required: false,
                deprecated: None,
                format: ParameterSchemaOrContent::Schema(String::schema_ref()),
                example: None,
                examples: Default::default(),
                explode: None,
                extensions: Default::default(),
            },
            kind: ParameterKind::Header {
                style: HeaderStyle::Simple,
            },
        })]
    }
}

impl FromRequest for Auth0Profile {
    type Error = Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(
            Self::from_request_impl(req)
                .inspect_err(|e| warn!("failed to extract auth0 profile: {e:?}")),
        )
    }
}

impl Auth0Profile {
    pub fn from_request_and_bearer(
        req: &actix_web::HttpRequest,
        bearer: BearerToken<'_>,
    ) -> Result<Self, Error> {
        // `Auth0State` should be provided on startup, so `unwrap` is safe
        let auth_state = req.app_data::<Data<Auth0State>>().unwrap();

        Ok(auth_state.decode::<Auth0Profile>(&bearer)?.claims)
    }

    pub fn from_request_impl(req: &actix_web::HttpRequest) -> Result<Self, Error> {
        // `Auth0State` should be provided on startup, so `unwrap` is safe
        let auth_state = req.app_data::<Data<Auth0State>>().unwrap();

        let bearer = BearerToken::from_request(req)?.ok_or(MissingJWTSnafu.build())?;

        Ok(auth_state.decode::<Auth0Profile>(&bearer)?.claims)
    }
}
