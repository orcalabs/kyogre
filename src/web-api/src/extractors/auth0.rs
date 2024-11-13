use actix_web::{web::Data, FromRequest};
use futures::future::{ready, Ready};
use kyogre_core::AisPermission;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use strum::EnumIter;
use tracing::warn;

use crate::{
    auth0::Auth0State,
    error::{
        error::{MissingJWTSnafu, ParseJWTSnafu},
        Error,
    },
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

impl From<Auth0Profile> for AisPermission {
    fn from(v: Auth0Profile) -> Self {
        if v.permissions.contains(&Auth0Permission::ReadAisUnder15m) {
            AisPermission::All
        } else {
            AisPermission::Above15m
        }
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
