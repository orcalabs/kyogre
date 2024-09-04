use std::pin::Pin;

use actix_web::FromRequest;
use futures::Future;
use kyogre_core::{AisPermission, BarentswatchUserId};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use strum::EnumIter;

use crate::{
    error::{
        bw_error::ProfileSnafu,
        error::{InvalidJWTSnafu, MissingJWTSnafu, ParseJWTSnafu},
        Error,
    },
    settings::BW_PROFILES_URL,
};

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
    pub id: BarentswatchUserId,
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
    type Error = Error;

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
                .ok_or_else(|| MissingJWTSnafu.build())?
                .context(ParseJWTSnafu)?;

            // This should always be set on application startup
            let url = BW_PROFILES_URL.get().unwrap();
            let client = reqwest::Client::new();

            let response = client
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await?;
            let status = response.status();
            match status {
                StatusCode::OK => {}
                StatusCode::UNAUTHORIZED => return InvalidJWTSnafu.fail(),
                _ => {
                    return Err(ProfileSnafu {
                        url,
                        status,
                        body: response.text().await?,
                    }
                    .build()
                    .into());
                }
            }

            let text = response.text().await?;

            Ok(serde_json::from_str(&text)?)
        })
    }
}
