use geo::geometry::Polygon;
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize,
};
use snafu::ResultExt;

use crate::{
    catch_location_id_error::{LengthSnafu, ParseSnafu},
    CatchLocationIdError,
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
    pub weather_location_ids: Vec<i64>,
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

impl TryFrom<&str> for CatchLocationId {
    type Error = CatchLocationIdError;

    fn try_from(v: &str) -> Result<Self, Self::Error> {
        let split: Vec<&str> = v.split('-').collect();
        if split.len() != 2 {
            return LengthSnafu { id: v.to_string() }.fail();
        }

        let main_area = split[0]
            .parse::<i32>()
            .with_context(|_| ParseSnafu { id: v.to_string() })?;

        let catch_area = split[1]
            .parse::<i32>()
            .with_context(|_| ParseSnafu { id: v.to_string() })?;

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
        CatchLocationId::try_from(v.as_ref())
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
