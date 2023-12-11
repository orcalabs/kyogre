use error_stack::{bail, Report};
use jurisdiction::Jurisdiction;
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new_unchecked(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
pub(crate) struct PrunedString(String);

impl AsRef<str> for NonEmptyString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = Report<Error>;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        NonEmptyString::try_from(value.to_owned())
    }
}
impl TryFrom<String> for NonEmptyString {
    type Error = Report<Error>;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            bail!(Error::Conversion);
        } else {
            Ok(NonEmptyString(value))
        }
    }
}

impl NonEmptyString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for PrunedString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl PartialEq<NonEmptyString> for String {
    fn eq(&self, other: &NonEmptyString) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'de> Deserialize<'de> for NonEmptyString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(NonEmptyStringVisitor)
    }
}

pub(crate) struct NonEmptyStringVisitor;
impl<'de> Visitor<'de> for NonEmptyStringVisitor {
    type Value = NonEmptyString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a non-empty string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        NonEmptyString::try_from(value)
            .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(value), &self))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        NonEmptyString::try_from(value.as_ref())
            .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(&value), &self))
    }
}

impl TryFrom<&str> for PrunedString {
    type Error = Report<Error>;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let pruned_value = value.replace(['_', '-', ' '], "");
        if pruned_value.is_empty() {
            bail!(Error::Conversion);
        } else {
            Ok(PrunedString(pruned_value))
        }
    }
}
impl TryFrom<String> for PrunedString {
    type Error = Report<Error>;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        PrunedString::try_from(value.as_str())
    }
}

impl PrunedString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<'de> Deserialize<'de> for PrunedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(PrunedStringVisitor)
    }
}

pub(crate) struct PrunedStringVisitor;
impl<'de> Visitor<'de> for PrunedStringVisitor {
    type Value = PrunedString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a non-empty string containing no '-', '_', or ' '")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        PrunedString::try_from(value)
            .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(value), &self))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        PrunedString::try_from(value.as_ref())
            .map_err(|_e| de::Error::invalid_value(de::Unexpected::Str(&value), &self))
    }
}

impl PartialEq<PrunedString> for String {
    fn eq(&self, other: &PrunedString) -> bool {
        other.as_ref().eq(self)
    }
}

impl From<Jurisdiction> for PrunedString {
    fn from(value: Jurisdiction) -> Self {
        PrunedString(value.alpha3().to_string())
    }
}
