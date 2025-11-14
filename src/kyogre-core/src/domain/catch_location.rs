use std::str::FromStr;

use fiskeridir_rs::sqlx_str_impl;
use geo::geometry::Polygon;
use serde::{
    Deserialize, Serialize,
    de::{self, Visitor},
};
use snafu::ResultExt;

use crate::{
    CatchLocationIdError,
    catch_location_id_error::{LengthSnafu, ParseSnafu},
};

pub enum WeatherLocationOverlap {
    OnlyOverlaps,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Ord, PartialOrd)]
#[serde(transparent)]
pub struct CatchLocationId {
    #[serde(skip)]
    main_area: i32,
    #[serde(skip)]
    catch_area: i32,
    val: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchLocation {
    pub id: CatchLocationId,
    pub polygon: Polygon,
    pub latitude: f64,
    pub longitude: f64,
}

impl std::fmt::Display for CatchLocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.val)
    }
}

impl CatchLocationId {
    pub fn new(main_area: i32, catch_area: i32) -> Self {
        Self {
            val: format!("{main_area:02}-{catch_area:02}"),
            main_area,
            catch_area,
        }
    }

    pub fn main_area(&self) -> i32 {
        self.main_area
    }

    pub fn catch_area(&self) -> i32 {
        self.catch_area
    }

    pub fn new_opt(main_area: Option<i32>, catch_area: Option<i32>) -> Option<Self> {
        match (main_area, catch_area) {
            (Some(main), Some(catch)) => Some(Self::new(main, catch)),
            _ => None,
        }
    }

    pub fn into_inner(self) -> String {
        self.val
    }
}

impl FromStr for CatchLocationId {
    type Err = CatchLocationIdError;

    fn from_str(v: &str) -> Result<Self, Self::Err> {
        let Some((main, catch)) = v.split_once('-') else {
            return LengthSnafu { id: v }.fail();
        };

        let main_area = main.parse().context(ParseSnafu { id: v })?;
        let catch_area = catch.parse().context(ParseSnafu { id: v })?;

        Ok(Self::new(main_area, catch_area))
    }
}

impl AsRef<str> for CatchLocationId {
    fn as_ref(&self) -> &str {
        &self.val
    }
}

impl TryFrom<String> for CatchLocationId {
    type Error = CatchLocationIdError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        v.parse()
    }
}

#[cfg(feature = "oasgen")]
oasgen::impl_oa_schema!(CatchLocationId, oasgen::Schema::new_string());

impl<'de> Deserialize<'de> for CatchLocationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(CatchLocationIdVisitor)
    }
}

struct CatchLocationIdVisitor;
impl Visitor<'_> for CatchLocationIdVisitor {
    type Value = CatchLocationId;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a valid catch location id")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value.parse().map_err(de::Error::custom)
    }
}

sqlx_str_impl!(CatchLocationId);
