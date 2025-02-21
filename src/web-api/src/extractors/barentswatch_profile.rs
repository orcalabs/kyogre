use std::{collections::HashMap, ops::Deref, pin::Pin};

use actix_web::{
    FromRequest,
    http::header::ToStrError,
    web::{self, Data},
};
use fiskeridir_rs::CallSign;
use futures::Future;
use http_client::{HttpClient, StatusCode};
use kyogre_core::{AisPermission, BarentswatchUserId};
use oasgen::{
    HeaderStyle, OaParameter, OaSchema, Parameter, ParameterData, ParameterKind,
    ParameterSchemaOrContent, RefOr,
};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, location};
use strum::EnumIter;
use tracing::warn;
use uuid::Uuid;

use crate::{
    error::{
        Error, Result,
        error::{MissingBwFiskInfoProfileSnafu, MissingJWTSnafu, ParseJWTSnafu},
    },
    settings::BW_PROFILES_URL,
    states::BwState,
};

static ORCA_ACCOUNT_ID: Uuid = parse_uuid("82c0012b-f337-47af-adc3-baaabce540a4");
static PER_GUNNAR_AURAN: Uuid = parse_uuid("6b01b65f-21e8-44b0-b3e3-9d547a217744");
static BAARD_JOHAN_HANSSEN: Uuid = parse_uuid("92d015cb-c10d-4748-b8d2-a4f4e27f2c64");
static PER_FINN: Uuid = parse_uuid("37999e6c-5e07-492a-b889-0ef3880e7009");
static ERLEND_STAV: Uuid = parse_uuid("6c1d8388-82c2-43d6-bb06-6b55f5b65fd7");
static TORE_SYVERSEN: Uuid = parse_uuid("0b3dce7f-233a-4450-a882-a69e06ea47e4");
static DORTHEA_VATN: Uuid = parse_uuid("85e96543-ff2f-483a-a86b-89c5554e0216");

static PROJECT_USERS: [Uuid; 7] = [
    ORCA_ACCOUNT_ID,
    PER_GUNNAR_AURAN,
    BAARD_JOHAN_HANSSEN,
    PER_FINN,
    ERLEND_STAV,
    TORE_SYVERSEN,
    DORTHEA_VATN,
];

const fn parse_uuid(uuid: &'static str) -> Uuid {
    match Uuid::try_parse(uuid) {
        Ok(u) => u,
        Err(_) => panic!("failed to parse uuid"),
    }
}

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
    pub ircs: CallSign,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BwProfile {
    pub user: BwUser,
    pub fisk_info_profile: Option<BwVesselInfo>,
    pub policies: Vec<BwPolicy>,
    pub roles: Vec<BwRole>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BwJwtClaims {
    #[serde(alias = "sub")]
    pub id: BarentswatchUserId,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionBwProfile(Option<BwProfile>);

impl OptionBwProfile {
    pub fn read_fishing_facilities(&self) -> bool {
        self.0
            .as_ref()
            .map(|p| {
                p.policies
                    .contains(&BwPolicy::BwReadExtendedFishingFacility)
            })
            .unwrap_or(false)
    }
}

impl OaParameter for BwProfile {
    fn parameters() -> Vec<RefOr<Parameter>> {
        vec![RefOr::Item(Parameter {
            data: ParameterData {
                name: "bw-token".into(),
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

impl OaParameter for OptionBwProfile {
    fn parameters() -> Vec<RefOr<Parameter>> {
        BwProfile::parameters()
            .into_iter()
            .flat_map(|v| v.into_item())
            .map(|mut v| {
                v.required = false;
                RefOr::Item(v)
            })
            .collect()
    }
}

impl From<&BwProfile> for AisPermission {
    fn from(value: &BwProfile) -> Self {
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
        // `BwState` should be provided on startup, so `unwrap` is safe
        let state = req.app_data::<Data<BwState>>().unwrap().clone();

        let token = req
            .headers()
            .get("bw-token")
            .map(|t| t.to_str().map(|s| s.to_owned()));
        let query_string = req.query_string().to_string();

        Box::pin(async move {
            BwProfile::extract_impl(state, client, token, query_string)
                .await
                .inspect_err(|e| warn!("failed to extract barentswatch profile: {e:?}"))
        })
    }
}

impl FromRequest for OptionBwProfile {
    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self>>>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let fut = BwProfile::from_request(req, payload);
        Box::pin(async move { Ok(Self(fut.await.ok())) })
    }
}

impl BwProfile {
    async fn extract_impl(
        state: Data<BwState>,
        client: Data<HttpClient>,
        token: Option<std::result::Result<String, ToStrError>>,
        query_string: String,
    ) -> Result<Self> {
        let token = token
            .ok_or_else(|| MissingJWTSnafu.build())?
            .context(ParseJWTSnafu)?;

        let claims = state.decode::<BwJwtClaims>(&token)?.claims;

        let mut profile = if let Some(profile) = state.get_profile(&claims.id).await {
            profile
        } else {
            // This should always be set on application startup
            let url = BW_PROFILES_URL.get().unwrap();

            let profile: BwProfile = client
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| match e.status() {
                    Some(StatusCode::UNAUTHORIZED) => Error::InvalidJWT {
                        location: location!(),
                        source: e,
                    },
                    _ => e.into(),
                })?
                .json()
                .await?;

            state.set_profile(profile.clone()).await;

            profile
        };

        if PROJECT_USERS.contains(profile.user.id.as_ref()) {
            let query: web::Query<HashMap<String, String>> = web::Query::from_query(&query_string)?;
            if let Some(cs) = query.get("call_sign_override") {
                profile.fisk_info_profile = Some(BwVesselInfo { ircs: cs.parse()? });
            }
        }

        Ok(profile)
    }

    pub fn call_sign(&self) -> Result<&CallSign> {
        self.fisk_info_profile
            .as_ref()
            .map(|v| &v.ircs)
            .ok_or_else(|| MissingBwFiskInfoProfileSnafu.build())
    }
}

impl OptionBwProfile {
    pub fn ais_permission(&self) -> AisPermission {
        self.0.as_ref().map(From::from).unwrap_or_default()
    }
}

impl Deref for OptionBwProfile {
    type Target = Option<BwProfile>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
