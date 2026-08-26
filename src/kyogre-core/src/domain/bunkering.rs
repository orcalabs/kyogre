use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct BunkeringId(i64);

#[derive(Debug, Clone, Deserialize, Serialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[serde(rename_all = "camelCase")]
pub struct Bunkering {
    pub id: BunkeringId,
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "fuel")]
    pub fuel_liter: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[serde(rename_all = "camelCase")]
pub struct CreateBunkering {
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "fuel")]
    pub fuel_liter: f64,
}

impl From<BunkeringId> for i64 {
    fn from(value: BunkeringId) -> Self {
        value.0
    }
}

impl std::fmt::Display for BunkeringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl BunkeringId {
    /// Need to construct this in a query
    pub fn new(value: i64) -> Self {
        Self(value)
    }
}
