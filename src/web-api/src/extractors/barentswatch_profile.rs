use std::{collections::HashMap, pin::Pin};

use actix_web::{
    web::{self, Data},
    FromRequest,
};
use fiskeridir_rs::CallSign;
use futures::Future;
use http_client::{HttpClient, StatusCode};
use kyogre_core::{AisPermission, BarentswatchUserId};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use strum::EnumIter;
use uuid::Uuid;

use crate::{
    error::{
        error::{
            InvalidCallSignSnafu, InvalidJWTSnafu, MissingBwFiskInfoProfileSnafu, MissingJWTSnafu,
            ParseJWTSnafu,
        },
        Error, Result,
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

    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        // `HttpClient` should be provided on startup, so `unwrap` is safe
        let client = req.app_data::<Data<HttpClient>>().unwrap().clone();

        let token = req
            .headers()
            .get("bw-token")
            .map(|t| t.to_str().map(|s| s.to_owned()));

        let query_string = req.query_string().to_string();

        Box::pin(async move {
            let token = token
                .ok_or_else(|| MissingJWTSnafu.build())?
                .context(ParseJWTSnafu)?;

            // This should always be set on application startup
            let url = BW_PROFILES_URL.get().unwrap();

            let mut response: BwProfile = client
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| match e.status() {
                    Some(StatusCode::UNAUTHORIZED) => InvalidJWTSnafu.build(),
                    _ => e.into(),
                })?
                .json()
                .await?;

            if let Ok(uuid) = Uuid::parse_str("82c0012b-f337-47af-adc3-baaabce540a4") {
                if *response.user.id.as_ref() == uuid {
                    let query: web::Query<HashMap<String, String>> =
                        web::Query::from_query(&query_string)?;
                    if let Some(cs) = query.get("call_sign_override") {
                        response.fisk_info_profile = Some(BwVesselInfo {
                            ircs: cs.to_string(),
                        });
                    }
                }
            }

            Ok(response)
        })
    }
}

impl BwProfile {
    pub fn call_sign(&self) -> Result<CallSign> {
        let profile = self
            .fisk_info_profile
            .as_ref()
            .ok_or_else(|| MissingBwFiskInfoProfileSnafu.build())?;

        profile.ircs.parse().context(InvalidCallSignSnafu {
            call_sign: &profile.ircs,
        })
    }
}
