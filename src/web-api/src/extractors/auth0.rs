use std::ops::Deref;

use actix_web::{http::header::AUTHORIZATION, web::Data, FromRequest};
use futures::future::{ready, Ready};
use kyogre_core::AisPermission;
use oasgen::{
    HeaderStyle, OaParameter, OaSchema, Parameter, ParameterData, ParameterKind,
    ParameterSchemaOrContent, RefOr,
};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use strum::EnumIter;
use tracing::warn;

use crate::{
    error::{
        error::{MissingJWTSnafu, ParseJWTSnafu},
        Error,
    },
    states::Auth0State,
};

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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionAuth0Profile(Option<Auth0Profile>);

impl From<Auth0Profile> for AisPermission {
    fn from(v: Auth0Profile) -> Self {
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
                required: true,
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

impl OaParameter for OptionAuth0Profile {
    fn parameters() -> Vec<RefOr<Parameter>> {
        Auth0Profile::parameters()
            .into_iter()
            .flat_map(|v| v.into_item())
            .map(|mut v| {
                v.required = false;
                RefOr::Item(v)
            })
            .collect()
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

impl FromRequest for OptionAuth0Profile {
    type Error = Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        ready(Ok(Self(
            Auth0Profile::from_request_impl(req)
                .inspect_err(|e| warn!("failed to extract auth0 profile: {e:?}"))
                .ok(),
        )))
    }
}

impl Auth0Profile {
    fn from_request_impl(req: &actix_web::HttpRequest) -> Result<Self, Error> {
        // `Auth0State` should be provided on startup, so `unwrap` is safe
        let auth_state = req.app_data::<Data<Auth0State>>().unwrap();

        match req
            .headers()
            .get("Authorization")
            .map(|t| {
                t.to_str()
                    .context(ParseJWTSnafu)
                    .map(|s| s.split_once(' ').map(|(_, t)| t.to_owned()))
            })
            .transpose()?
            .flatten()
        {
            Some(t) => Ok(auth_state.decode::<Auth0Profile>(&t)?.claims),
            None => MissingJWTSnafu.fail(),
        }
    }
}

impl OptionAuth0Profile {
    pub fn into_inner(self) -> Option<Auth0Profile> {
        self.0
    }
}

impl Deref for OptionAuth0Profile {
    type Target = Option<Auth0Profile>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
