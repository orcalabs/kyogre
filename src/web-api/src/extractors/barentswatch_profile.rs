use std::pin::Pin;

use actix_web::FromRequest;
use futures::Future;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tracing::{event, Level};
use uuid::Uuid;

use crate::{error::ApiError, settings::BW_PROFILES_URL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, EnumIter)]
pub enum BwPolicy {
    BwReadExtendedFishingFacility,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BwProfile {
    pub id: Uuid,
    pub policies: Vec<BwPolicy>,
}

impl FromRequest for BwProfile {
    type Error = ApiError;

    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let token = req
            .headers()
            .get("bw-token")
            .map(|t| t.to_str().map(|s| s.to_owned()));

        Box::pin(async move {
            let token = token
                .ok_or(ApiError::MissingBwToken)?
                .map_err(|_| ApiError::InvalidBwToken)?;

            let url = BW_PROFILES_URL.get().ok_or(ApiError::InternalServerError)?;
            let client = reqwest::Client::new();

            let response = client
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| {
                    event!(Level::WARN, "request to barentswatch failed: {:?}", e);
                    ApiError::InternalServerError
                })?;
            match response.status() {
                StatusCode::OK => {}
                StatusCode::UNAUTHORIZED => return Err(ApiError::InvalidBwToken),
                _ => {
                    event!(
                        Level::WARN,
                        "unexpected response from barentswatch: {:?}",
                        response
                    );
                    return Err(ApiError::InternalServerError);
                }
            }

            let profile: BwProfile = response.json().await.map_err(|e| {
                event!(
                    Level::WARN,
                    "deserializing barentswatch response failed: {:?}",
                    e
                );
                ApiError::InternalServerError
            })?;

            Ok(profile)
        })
    }
}
