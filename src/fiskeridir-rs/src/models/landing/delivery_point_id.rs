use std::{ops::Deref, str::FromStr};

use crate::{
    error::ParseStringError,
    sqlx_str_impl,
    string_new_types::{PrunedString, PrunedStringVisitor},
};
use jurisdiction::Jurisdiction;
use serde::{Deserialize, Serialize};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

/// NewType wrapper for delivery point ids, enforces that delivery point ids cannot contain
/// '_', '-', ' ' or be empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct DeliveryPointId(PrunedString);

impl DeliveryPointId {
    pub fn into_inner(self) -> String {
        self.0.into_inner()
    }

    /// Returns wheter this delivery point is a broenbaat
    pub fn broenbaat(&self) -> bool {
        self.0.as_ref().ends_with("BRB") || self.0.as_ref().ends_with("brb")
    }

    /// Creates a new DeliveryPointId and panic if its invalid
    pub fn new_unchecked<T: ToString>(val: T) -> DeliveryPointId {
        let val = val.to_string();
        DeliveryPointId::try_from(val).unwrap()
    }
}

impl AsRef<str> for DeliveryPointId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Deref for DeliveryPointId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl FromStr for DeliveryPointId {
    type Err = ParseStringError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

impl TryFrom<String> for DeliveryPointId {
    type Error = ParseStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl PartialEq<DeliveryPointId> for String {
    fn eq(&self, other: &DeliveryPointId) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'de> Deserialize<'de> for DeliveryPointId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_str(PrunedStringVisitor)
            .map(DeliveryPointId)
    }
}

impl From<Jurisdiction> for DeliveryPointId {
    fn from(value: Jurisdiction) -> Self {
        DeliveryPointId(PrunedString::from(value))
    }
}

sqlx_str_impl!(DeliveryPointId);
