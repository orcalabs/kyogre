use crate::{
    CallSign,
    error::{ParseStringError, parse_string_error::EmptySnafu},
};
use jurisdiction::Jurisdiction;
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use std::{fmt::Display, ops::Deref, str::FromStr};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new_unchecked(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OptPrunedString(Option<PrunedString>);

impl From<OptPrunedString> for Option<CallSign> {
    fn from(value: OptPrunedString) -> Self {
        value.0.map(CallSign::from)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub(crate) struct PrunedString(String);

impl AsRef<str> for NonEmptyString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Deref for NonEmptyString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl FromStr for NonEmptyString {
    type Err = ParseStringError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            EmptySnafu.fail()
        } else {
            Ok(Self(value.into()))
        }
    }
}

impl NonEmptyString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for PrunedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
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
impl Visitor<'_> for NonEmptyStringVisitor {
    type Value = NonEmptyString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a non-empty string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value
            .parse()
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(value), &self))
    }
}

impl FromStr for PrunedString {
    type Err = ParseStringError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let pruned_value = value.trim().replace(['_', '-', ' '], "");
        if pruned_value.is_empty() {
            EmptySnafu.fail()
        } else {
            Ok(PrunedString(pruned_value))
        }
    }
}

impl TryFrom<String> for PrunedString {
    type Error = ParseStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
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

impl<'de> Deserialize<'de> for OptPrunedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_option(OptPrunedStringVisitor)
    }
}

pub(crate) struct OptPrunedStringVisitor;
impl<'a> Visitor<'a> for OptPrunedStringVisitor {
    type Value = OptPrunedString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a non-empty string containing no '-', '_', or ' '")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(OptPrunedString(PrunedString::from_str(value).ok()))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'a>,
    {
        deserializer.deserialize_str(OptPrunedStringVisitor)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(OptPrunedString(None))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(OptPrunedString(None))
    }
}

pub(crate) struct PrunedStringVisitor;
impl Visitor<'_> for PrunedStringVisitor {
    type Value = PrunedString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a non-empty string containing no '-', '_', or ' '")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value
            .parse()
            .map_err(|_| de::Error::invalid_value(de::Unexpected::Str(value), &self))
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

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for PrunedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
