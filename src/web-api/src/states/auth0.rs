use crate::{
    error::{
        JWTDecodeError,
        jwt_decode_error::{DisabledSnafu, MissingValueSnafu},
    },
    settings::Auth0Settings,
};
use http_client::HttpClient;
use jsonwebtoken::{
    Algorithm, DecodingKey, TokenData, Validation, decode, decode_header, jwk::JwkSet,
};
use serde::de::DeserializeOwned;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Auth0State {
    Disabled,
    Enabled { jwk_set: JwkSet, audience: String },
}

impl Auth0State {
    pub async fn new(settings: Option<&Auth0Settings>) -> Self {
        if let Some(settings) = settings {
            let jwk_set: JwkSet = HttpClient::new()
                .get(&settings.jwk_url)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();

            Self::Enabled {
                jwk_set,
                audience: settings.audience.clone(),
            }
        } else {
            Self::Disabled
        }
    }

    pub fn decode<T: DeserializeOwned>(&self, token: &str) -> Result<TokenData<T>, JWTDecodeError> {
        let Self::Enabled { jwk_set, audience } = self else {
            return DisabledSnafu {}.fail();
        };

        let header = decode_header(token)?;
        let kid = header.kid.ok_or_else(|| MissingValueSnafu.build())?;

        let jwk = jwk_set
            .find(&kid)
            .ok_or_else(|| MissingValueSnafu.build())?;

        let key = DecodingKey::from_jwk(jwk)?;
        let mut validation = Validation::new(Algorithm::from_str(
            jwk.common
                .key_algorithm
                .ok_or_else(|| MissingValueSnafu.build())?
                .to_string()
                .as_str(),
        )?);
        validation.set_audience(&[&audience]);

        Ok(decode(token, &key, &validation)?)
    }
}
