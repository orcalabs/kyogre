use std::str::FromStr;

use error_stack::{bail, report, Result, ResultExt};
use jsonwebtoken::{
    decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, TokenData, Validation,
};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub enum Auth0State {
    Disabled,
    Enabled { jwk_set: JwkSet },
}

impl Auth0State {
    pub async fn new(jwk_url: Option<String>) -> Self {
        if let Some(ref url) = jwk_url {
            let jwk_set: JwkSet = reqwest::get(url).await.unwrap().json().await.unwrap();
            Self::Enabled { jwk_set }
        } else {
            Self::Disabled
        }
    }

    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, Auth0Error> {
        let jwk_set = match self {
            Self::Enabled { jwk_set, .. } => jwk_set,
            Self::Disabled => bail!(Auth0Error::DecodeDisabled),
        };

        let header = decode_header(token).change_context(Auth0Error::DecodeHeader)?;
        let kid = header
            .kid
            .ok_or_else(|| report!(Auth0Error::MissingKidInHeader))?;

        let jwk = jwk_set
            .find(&kid)
            .ok_or_else(|| report!(Auth0Error::MissingKidInJwkSet))?;

        let key = DecodingKey::from_jwk(jwk).change_context(Auth0Error::DecodeKeyFromJwk)?;
        let validation = Validation::new(
            Algorithm::from_str(
                jwk.common
                    .key_algorithm
                    .ok_or_else(|| report!(Auth0Error::MissingAlgorithmInJwk))?
                    .to_string()
                    .as_str(),
            )
            .change_context(Auth0Error::InvalidAlgorithmInJwk)?,
        );

        decode(token, &key, &validation).change_context(Auth0Error::DecodeToken)
    }
}

#[derive(Debug)]
pub enum Auth0Error {
    DecodeDisabled,
    DecodeHeader,
    DecodeToken,
    DecodeKeyFromJwk,
    MissingKidInHeader,
    MissingKidInJwkSet,
    MissingAlgorithmInJwk,
    InvalidAlgorithmInJwk,
}

impl std::error::Error for Auth0Error {}

impl std::fmt::Display for Auth0Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Auth0Error::DecodeDisabled => {
                f.write_str("tried to decode a token with at disabled Auth0Guard")
            }
            Auth0Error::DecodeHeader => f.write_str("failed to decode JWT header"),
            Auth0Error::DecodeToken => f.write_str("failed to decode JWT token"),
            Auth0Error::DecodeKeyFromJwk => f.write_str("failed to parse decoding key from JWK"),
            Auth0Error::MissingKidInHeader => f.write_str("kid missing in header"),
            Auth0Error::MissingKidInJwkSet => f.write_str("kid missing in JWK Set"),
            Auth0Error::MissingAlgorithmInJwk => f.write_str("algorithm missing in JWK"),
            Auth0Error::InvalidAlgorithmInJwk => f.write_str("invalid algorithm in JWK"),
        }
    }
}
