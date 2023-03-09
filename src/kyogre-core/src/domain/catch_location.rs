use error_stack::{report, IntoReport, Report, ResultExt};
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CatchLocationId(String);

impl CatchLocationId {
    pub fn new(main_area: i32, catch_area: i32) -> Self {
        Self(format!("{main_area:02}-{catch_area:02}"))
    }

    pub fn new_opt(main_area: Option<i32>, catch_area: Option<i32>) -> Option<Self> {
        match (main_area, catch_area) {
            (Some(main), Some(catch)) => Some(Self::new(main, catch)),
            _ => None,
        }
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug)]
pub enum CatchLocationIdError {
    InvalidLength,
    InvalidMainArea,
    InvalidCatchArea,
}

impl std::error::Error for CatchLocationIdError {}

impl std::fmt::Display for CatchLocationIdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatchLocationIdError::InvalidLength => {
                f.write_str("catch location id did not contain a valid length")
            }
            CatchLocationIdError::InvalidMainArea => {
                f.write_str("catch location id did not contain a valid main area")
            }
            CatchLocationIdError::InvalidCatchArea => {
                f.write_str("catch location id did not contain a valid catch area")
            }
        }
    }
}

impl TryFrom<&str> for CatchLocationId {
    type Error = Report<CatchLocationIdError>;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let split: Vec<&str> = v.split('-').collect();
        if split.len() != 2 {
            return Err(report!(CatchLocationIdError::InvalidLength));
        }

        let main_area = split[0]
            .parse()
            .into_report()
            .change_context(CatchLocationIdError::InvalidMainArea)?;

        let catch_area = split[1]
            .parse()
            .into_report()
            .change_context(CatchLocationIdError::InvalidCatchArea)?;

        Ok(Self::new(main_area, catch_area))
    }
}

impl TryFrom<String> for CatchLocationId {
    type Error = Report<CatchLocationIdError>;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        CatchLocationId::try_from(v.as_ref())
    }
}

impl ToString for CatchLocationId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl<'de> Deserialize<'de> for CatchLocationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(CatchLocationIdVisitor)
    }
}

struct CatchLocationIdVisitor;
impl<'de> Visitor<'de> for CatchLocationIdVisitor {
    type Value = CatchLocationId;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a valid catch location id")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        CatchLocationId::try_from(value).map_err(|e| de::Error::custom(e))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        CatchLocationId::try_from(value).map_err(|e| de::Error::custom(e))
    }
}
