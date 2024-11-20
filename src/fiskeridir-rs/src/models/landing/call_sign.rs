use std::{fmt::Display, ops::Deref, str::FromStr};

use crate::{
    error::ParseStringError,
    sqlx_str_impl,
    string_new_types::{PrunedString, PrunedStringVisitor},
};
use serde::{Deserialize, Serialize};

/// NewType wrapper for call signs, enforces that call signs cannot contain
/// '_', '-', ' ' or be empty.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
pub struct CallSign(PrunedString);

impl CallSign {
    /// Creates a new CallSign and panic if its invalid
    pub fn new_unchecked<T: ToString>(val: T) -> CallSign {
        let val = val.to_string();
        CallSign::try_from(val).unwrap()
    }
    pub fn into_inner(self) -> String {
        self.0.into_inner()
    }
}

impl Display for CallSign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for CallSign {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl AsRef<str> for CallSign {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl FromStr for CallSign {
    type Err = ParseStringError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

impl TryFrom<&str> for CallSign {
    type Error = ParseStringError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for CallSign {
    type Error = ParseStringError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl PartialEq<CallSign> for String {
    fn eq(&self, other: &CallSign) -> bool {
        other.as_ref().eq(self)
    }
}

impl<'de> Deserialize<'de> for CallSign {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_str(PrunedStringVisitor)
            .map(CallSign)
    }
}

sqlx_str_impl!(CallSign);
