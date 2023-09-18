use std::collections::HashMap;

use actix_web::guard::Guard;
use error_stack::{report, Result, ResultExt};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{Jwk, JwkSet},
    DecodingKey, TokenData, Validation,
};
use serde::de::DeserializeOwned;
use tracing::{event, Level};

#[derive(Debug, Clone)]
pub struct JwtGuard {
    jwks: HashMap<String, Jwk>,
}

impl JwtGuard {
    pub async fn new(jwks_url: String) -> Self {
        let jwks: JwkSet = reqwest::get(jwks_url).await.unwrap().json().await.unwrap();

        JwtGuard {
            jwks: jwks
                .keys
                .into_iter()
                .filter_map(|k| k.common.key_id.clone().map(|kid| (kid, k)))
                .collect(),
        }
    }

    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, JwtDecodeError> {
        let kid = decode_header(token)
            .change_context(JwtDecodeError::DecodeHeader)?
            .kid
            .ok_or_else(|| report!(JwtDecodeError::MissingKidInHeaders))?;

        match self.jwks.get(&kid) {
            Some(jwk) => {
                let key =
                    DecodingKey::from_jwk(jwk).change_context(JwtDecodeError::DecodeKeyFromJwk)?;

                let validation = Validation::new(
                    jwk.common
                        .algorithm
                        .ok_or_else(|| report!(JwtDecodeError::MissingAlgorithmInJwk))?,
                );

                decode::<T>(token, &key, &validation).change_context(JwtDecodeError::DecodeToken)
            }
            None => Err(report!(JwtDecodeError::MissingKidInJwk)),
        }
    }
}

impl Guard for JwtGuard {
    fn check(&self, ctx: &actix_web::guard::GuardContext<'_>) -> bool {
        match ctx.head().headers.get("bw-token") {
            Some(token) => match token.to_str() {
                Ok(token) => self
                    .decode::<serde_json::Value>(token)
                    .map_err(|e| {
                        event!(Level::WARN, "failed to decode token: {token}, err: {:?}", e);
                        e
                    })
                    .is_ok(),
                Err(_) => false,
            },
            None => false,
        }
    }
}

#[derive(Debug)]
pub enum JwtDecodeError {
    DecodeHeader,
    DecodeToken,
    DecodeKeyFromJwk,
    MissingKidInHeaders,
    MissingAlgorithmInJwk,
    MissingKidInJwk,
}

impl std::error::Error for JwtDecodeError {}

impl std::fmt::Display for JwtDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JwtDecodeError::DecodeHeader => f.write_str("failed to decode token header"),
            JwtDecodeError::DecodeToken => f.write_str("failed to decode token"),
            JwtDecodeError::DecodeKeyFromJwk => {
                f.write_str("failed to convert jwk to decoding key")
            }
            JwtDecodeError::MissingKidInHeaders => f.write_str("kid missing in headers"),
            JwtDecodeError::MissingAlgorithmInJwk => f.write_str("algorithm missing in jwk"),
            JwtDecodeError::MissingKidInJwk => f.write_str("kid missing in jwk"),
        }
    }
}
