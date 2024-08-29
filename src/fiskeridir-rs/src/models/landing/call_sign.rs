use crate::{
    error::ParseStringError,
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

impl AsRef<str> for CallSign {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl TryFrom<&str> for CallSign {
    type Error = ParseStringError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        PrunedString::try_from(value).map(CallSign)
    }
}

impl TryFrom<String> for CallSign {
    type Error = ParseStringError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        PrunedString::try_from(value).map(CallSign)
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
