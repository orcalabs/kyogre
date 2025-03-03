use std::{collections::HashMap, str::FromStr, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use http_client::HttpClient;
use jsonwebtoken::{
    Algorithm, DecodingKey, TokenData, Validation, decode, decode_header,
    jwk::{Jwk, JwkSet},
};
use kyogre_core::BarentswatchUserId;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::{
    error::{
        JWTDecodeError,
        jwt_decode_error::{DisabledSnafu, MissingValueSnafu},
    },
    extractors::{BearerToken, BwProfile},
    guards::BwGuard,
    settings::BwSettings,
};

static BW_PROFILE_CACHE_TTL: Duration = Duration::hours(24);

#[derive(Debug, Clone)]
pub enum BwState {
    Disabled,
    Enabled(Arc<Inner>),
}

#[derive(Debug)]
pub struct Inner {
    jwks: HashMap<String, Jwk>,
    audience: String,
    cache: RwLock<HashMap<BarentswatchUserId, CacheItem>>,
}

#[derive(Debug)]
struct CacheItem {
    profile: BwProfile,
    expires: DateTime<Utc>,
}

impl BwState {
    pub async fn new(settings: Option<&BwSettings>) -> Self {
        if let Some(settings) = settings {
            let jwks: JwkSet = HttpClient::new()
                .get(&settings.jwks_url)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            Self::Enabled(Arc::new(Inner {
                jwks: jwks
                    .keys
                    .into_iter()
                    .filter_map(|k| k.common.key_id.clone().map(|kid| (kid, k)))
                    .collect(),
                audience: settings.audience.clone(),
                cache: Default::default(),
            }))
        } else {
            Self::Disabled
        }
    }

    pub fn guard(&self) -> Option<BwGuard> {
        match self {
            Self::Enabled(v) => Some(BwGuard::new(Self::Enabled(v.clone()))),
            Self::Disabled => None,
        }
    }

    pub fn decode<T: DeserializeOwned>(
        &self,
        token: &BearerToken<'_>,
    ) -> Result<TokenData<T>, JWTDecodeError> {
        match self {
            Self::Enabled(v) => v.decode(token),
            Self::Disabled => DisabledSnafu.fail(),
        }
    }

    pub async fn get_profile(&self, id: &BarentswatchUserId) -> Option<BwProfile> {
        match self {
            Self::Enabled(v) => v.get_profile(id).await,
            Self::Disabled => None,
        }
    }

    pub async fn set_profile(&self, profile: BwProfile) {
        match self {
            Self::Enabled(v) => v.set_profile(profile).await,
            Self::Disabled => {}
        }
    }
}

impl Inner {
    fn decode<T: DeserializeOwned>(
        &self,
        token: &BearerToken<'_>,
    ) -> Result<TokenData<T>, JWTDecodeError> {
        let token = token.token();
        let kid = decode_header(token)?
            .kid
            .ok_or_else(|| MissingValueSnafu.build())?;

        let jwk = self
            .jwks
            .get(&kid)
            .ok_or_else(|| MissingValueSnafu.build())?;
        let key = DecodingKey::from_jwk(jwk)?;

        let mut validation = Validation::new(Algorithm::from_str(
            jwk.common
                .key_algorithm
                .ok_or_else(|| MissingValueSnafu.build())?
                .to_string()
                .as_str(),
        )?);
        validation.set_audience(&[&self.audience]);

        Ok(decode::<T>(token, &key, &validation)?)
    }

    pub async fn get_profile(&self, id: &BarentswatchUserId) -> Option<BwProfile> {
        self.cache
            .read()
            .await
            .get(id)
            .and_then(|v| (v.expires > Utc::now()).then(|| v.profile.clone()))
    }

    pub async fn set_profile(&self, profile: BwProfile) {
        self.cache.write().await.insert(
            profile.user.id,
            CacheItem {
                profile,
                expires: Utc::now() + BW_PROFILE_CACHE_TTL,
            },
        );
    }
}
