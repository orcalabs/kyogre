use std::fmt;

use crate::Error;
use crate::{DocumentType, SalesTeam};
use num_traits::FromPrimitive;

use error_stack::{report, Report, ResultExt};
use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};

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

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                LandingId::try_from(value)
                    .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(value), &self))
            }
            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                LandingId::try_from(value.as_str())
                    .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(&value), &self))
            }
        }

        deserializer.deserialize_str(LandigIdVisitor)
    }
}

impl TryFrom<&str> for LandingId {
    type Error = Report<Error>;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let split: Vec<&str> = value.split('-').collect();
        if split.len() != 4 {
            return Err(report!(Error::Conversion));
        }

        let document_id: i64 = split[0].parse::<i64>().change_context(Error::Conversion)?;
        let sale_team_id: SalesTeam =
            SalesTeam::from_i32(split[1].parse::<i32>().change_context(Error::Conversion)?)
                .ok_or_else(|| report!(Error::Conversion))?;
        let document_type: DocumentType =
            DocumentType::from_i32(split[2].parse::<i32>().change_context(Error::Conversion)?)
                .ok_or_else(|| report!(Error::Conversion))?;
        let year: u32 = split[3].parse::<u32>().change_context(Error::Conversion)?;

        Ok(LandingId(format!(
            "{}-{}-{}-{}",
            document_id, sale_team_id as i32, document_type as i32, year
        )))
    }
}

impl TryFrom<String> for LandingId {
    type Error = Report<Error>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        LandingId::try_from(value.as_str())
    }
}

impl LandingId {
    /// Consume the `LandingId` and returned the wrapped value.
    pub fn into_inner(self) -> String {
        self.0
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
