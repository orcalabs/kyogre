use std::{collections::HashMap, str::FromStr};

use actix_web::guard::Guard;
use http_client::HttpClient;
use jsonwebtoken::{
    decode, decode_header,
    jwk::{Jwk, JwkSet},
    Algorithm, DecodingKey, TokenData, Validation,
};
use serde::de::DeserializeOwned;
use tracing::warn;

use crate::{
    error::{jwt_decode_error::MissingValueSnafu, JWTDecodeError},
    settings::BwSettings,
};

#[derive(Debug, Clone)]
pub struct BwtGuard {
    jwks: HashMap<String, Jwk>,
    audience: String,
}

impl BwtGuard {
    pub async fn new(settings: &BwSettings) -> Self {
        let jwks: JwkSet = HttpClient::new()
            .get(&settings.jwks_url)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        BwtGuard {
            jwks: jwks
                .keys
                .into_iter()
                .filter_map(|k| k.common.key_id.clone().map(|kid| (kid, k)))
                .collect(),
            audience: settings.audience.clone(),
        }
    }

    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, JWTDecodeError> {
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
}

impl Guard for BwtGuard {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        match ctx.head().headers.get("bw-token") {
            Some(token) => match token.to_str() {
                Ok(token) => self
                    .decode::<serde_json::Value>(token)
                    .map_err(|e| {
                        warn!("failed to decode token: {token}, err: {e:?}");
                        e
                    })
                    .is_ok(),
                Err(_) => false,
            },
            None => false,
        }
    }
}
