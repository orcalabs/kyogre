use std::pin::Pin;

use actix_web::FromRequest;
use futures::Future;
use kyogre_core::AisPermission;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tracing::{event, Level};
use uuid::Uuid;

use crate::{error::ApiError, settings::BW_PROFILES_URL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, EnumIter)]
pub enum BwPolicy {
    BwReadExtendedFishingFacility,
    BwAisFiskinfo,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, EnumIter)]
pub enum BwRole {
    BwDownloadFishingfacility,
    BwEksternFiskInfoUtvikler,
    BwFiskerikyndig,
    BwFiskinfoAdmin,
    BwUtdanningsBruker,
    BwViewAis,
    BwYrkesfisker,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BwUser {
    pub id: Uuid,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BwVesselInfo {
    pub ircs: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BwProfile {
    pub user: BwUser,
    pub fisk_info_profile: Option<BwVesselInfo>,
    pub policies: Vec<BwPolicy>,
    pub roles: Vec<BwRole>,
}

impl From<BwProfile> for AisPermission {
    fn from(value: BwProfile) -> Self {
        let ais_policy = value.policies.iter().any(|v| *v == BwPolicy::BwAisFiskinfo);
        if ais_policy {
            value
                .roles
                .iter()
                .find(|v| match v {
                    BwRole::BwDownloadFishingfacility
                    | BwRole::BwEksternFiskInfoUtvikler
                    | BwRole::BwFiskerikyndig
                    | BwRole::BwFiskinfoAdmin
                    | BwRole::BwUtdanningsBruker
                    | BwRole::BwViewAis
                    | BwRole::BwYrkesfisker => true,
                    BwRole::Other => false,
                })
                .map(|_| AisPermission::All)
                .unwrap_or_default()
        } else {
            AisPermission::default()
        }
    }
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

            let text = response.text().await.map_err(|e| {
                event!(
                    Level::WARN,
                    "failed to parse barentswatch response text, err: {:?}",
                    e
                );
                ApiError::InternalServerError
            })?;

            serde_json::from_str(&text).map_err(|e| {
                event!(
                    Level::WARN,
                    "failed to deserialize BwProfile from {text}, err: {:?}",
                    e
                );
                ApiError::InternalServerError
            })
        })
    }
}
