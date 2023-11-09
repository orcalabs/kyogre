use actix_web::{web::Data, FromRequest};
use futures::future::{ready, Ready};
use kyogre_core::AisPermission;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tracing::{event, Level};

use crate::{auth0::Auth0State, error::ApiError};

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
    type Error = ApiError;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        // `Auth0State` should be provided on startup, so `unwrap` is safe
        let auth_state = req.app_data::<Data<Auth0State>>().unwrap();

        let token = match req
            .headers()
            .get("Authorization")
            .map(|t| {
                t.to_str()
                    .map_err(|_| ApiError::InvalidAuthToken)
                    .map(|s| s.split_once(' ').map(|(_, t)| t.to_owned()))
            })
            .transpose()
        {
            Ok(t) => t,
            Err(e) => return ready(Err(e)),
        };

        let auth = match token.flatten() {
            Some(t) => auth_state
                .decode::<Auth0Profile>(&t)
                .map_err(|e| {
                    event!(Level::WARN, "failed to decode token: err: {e:?}");
                    ApiError::InvalidAuthToken
                })
                .map(|v| v.claims),
            None => Err(ApiError::MissingAuthToken),
        };

        ready(auth)
    }
}
