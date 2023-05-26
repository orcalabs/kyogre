use std::collections::HashMap;

use actix_web::guard::Guard;
use error_stack::{report, Context, IntoReport, Result, ResultExt};
use jsonwebtoken::{decode, decode_header, jwk::Jwk, DecodingKey, TokenData, Validation};
use serde::{de::DeserializeOwned, Deserialize};
use tracing::{event, Level};

#[derive(Debug, Clone)]
pub struct JwtGuard {
    jwks: HashMap<String, Jwk>,
}

impl JwtGuard {
    pub async fn new(jwks_url: String) -> Self {
        let jwks: Jwks = reqwest::get(jwks_url).await.unwrap().json().await.unwrap();

        JwtGuard {
            jwks: jwks
                .keys
                .into_iter()
                .filter_map(|mut k| k.common.key_id.take().map(|kid| (kid, k)))
                .collect(),
        }
    }

    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, JwtDecodeError> {
        let kid = decode_header(token)
            .into_report()
            .change_context(JwtDecodeError)?
            .kid
            .ok_or_else(|| {
                report!(JwtDecodeError).attach_printable("kid missing in token headers")
            })?;

        match self.jwks.get(&kid) {
            Some(jwk) => {
                let key = DecodingKey::from_jwk(jwk)
                    .into_report()
                    .change_context(JwtDecodeError)?;

                let validation = Validation::new(jwk.common.algorithm.ok_or_else(|| {
                    report!(JwtDecodeError).attach_printable("algorithm missing in jwk")
                })?);

                decode::<T>(token, &key, &validation)
                    .into_report()
                    .change_context(JwtDecodeError)
            }
            None => Err(report!(JwtDecodeError).attach_printable("kid not found in jwks")),
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

#[derive(Debug, Clone, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug)]
pub struct JwtDecodeError;

impl Context for JwtDecodeError {}

impl std::fmt::Display for JwtDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an error occurred during JWT decoding")
    }
}
