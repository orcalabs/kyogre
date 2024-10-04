use crate::error::landing_id_error::{InvalidSnafu, LengthSnafu, ParseSnafu};
use crate::error::LandingIdError;
use crate::{sqlx_str_impl, DocumentType, SalesTeam};
use core::fmt;
use num_traits::FromPrimitive;
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use std::str::FromStr;

/// NewType wrapping the creation of unique landing ids.
#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug, Hash, Serialize)]
pub struct LandingId(String);

impl LandingId {
    /// Each sale team have a running serial key as document id which is shared across all document
    /// types.
    /// Before 2018 this serial key is only unique within a single year.
    /// To create a unique we have to combine these four fields, see the official
    /// Fiskedirektoratet landing docs for more details.
    pub fn new(
        document_id: i64,
        sale_team_orginization_code: SalesTeam,
        document_type: DocumentType,
        year: u32,
    ) -> LandingId {
        LandingId(format!(
            "{}-{}-{}-{}",
            document_id, sale_team_orginization_code as u32, document_type as u32, year
        ))
    }

    /// Consume the `LandingId` and returned the wrapped value.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for LandingId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct LandigIdVisitor;

        impl<'de> Visitor<'de> for LandigIdVisitor {
            type Value = LandingId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid landing id")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.parse()
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_str(LandigIdVisitor)
    }
}

impl FromStr for LandingId {
    type Err = LandingIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = value.split('-').collect();
        if split.len() != 4 {
            return LengthSnafu { value }.fail();
        }

        let document_id = split[0].parse().context(ParseSnafu { value })?;

        let sale_team_id = SalesTeam::from_i32(split[1].parse().context(ParseSnafu { value })?)
            .ok_or_else(|| InvalidSnafu { value: split[1] }.build())?;

        let document_type = DocumentType::from_i32(split[2].parse().context(ParseSnafu { value })?)
            .ok_or_else(|| InvalidSnafu { value: split[2] }.build())?;

        let year = split[3].parse().context(ParseSnafu { value })?;

        Ok(LandingId::new(
            document_id,
            sale_team_id,
            document_type,
            year,
        ))
    }
}

impl TryFrom<String> for LandingId {
    type Error = LandingIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl AsRef<str> for LandingId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LandingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

sqlx_str_impl!(LandingId);
