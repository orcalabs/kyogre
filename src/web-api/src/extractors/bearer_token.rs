use crate::error::{
    Result,
    error::{InvalidJWTPartsSnafu, ParseJWTSnafu},
};
use actix_web::http::header::{AUTHORIZATION, HeaderMap};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{
    Deserialize,
    de::{self, Visitor},
};
use snafu::ResultExt;
use std::borrow::Cow;
use std::str::FromStr;

#[derive(Debug)]
pub struct BearerToken<'a>(Cow<'a, str>);

#[derive(Debug, Clone, Copy, strum::EnumString, strum::Display)]
pub enum AcceptedIssuer {
    #[strum(to_string = "https://dev-orcalabs.eu.auth0.com/")]
    OrcaDev,
    #[strum(to_string = "https://id.barentswatch.no")]
    Barentswatch,
}

impl<'a> BearerToken<'a> {
    pub fn token(&self) -> &str {
        &self.0
    }

    pub fn issuer(&self) -> Result<AcceptedIssuer> {
        let split: Vec<&str> = self.0.split('.').collect();
        if split.len() != 3 {
            return InvalidJWTPartsSnafu.fail();
        }
        let decoded = URL_SAFE_NO_PAD.decode(split[1])?;

        #[derive(Deserialize)]
        struct JWTIssuer {
            iss: AcceptedIssuer,
        }
        let issuer: JWTIssuer = serde_json::from_slice(&decoded)?;

        Ok(issuer.iss)
    }

    pub fn from_request_inner(headers: &HeaderMap) -> Result<Option<&str>> {
        Ok(headers
            .get(AUTHORIZATION)
            .map(|t| {
                t.to_str()
                    .context(ParseJWTSnafu)
                    .map(|s| s.split_once(' ').map(|(_, t)| t))
            })
            .transpose()?
            .flatten())
    }
    pub fn from_request(req: &'a actix_web::HttpRequest) -> Result<Option<Self>> {
        Ok(Self::from_request_inner(req.headers())?.map(|h| Self(Cow::Borrowed(h))))
    }

    pub fn from_request_owned(req: &actix_web::HttpRequest) -> Result<Option<Self>> {
        Ok(Self::from_request_inner(req.headers())?.map(|h| Self(Cow::Owned(h.to_owned()))))
    }

    pub fn from_guard_context(ctx: &'a actix_web::guard::GuardContext<'_>) -> Result<Option<Self>> {
        Ok(Self::from_request_inner(ctx.head().headers())?.map(|v| Self(Cow::Borrowed(v))))
    }
}

impl<'de> Deserialize<'de> for AcceptedIssuer {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct IssuerVisitor;

        impl Visitor<'_> for IssuerVisitor {
            type Value = AcceptedIssuer;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an accepted JWT issuer")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                AcceptedIssuer::from_str(v)
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(IssuerVisitor)
    }
}
