use crate::NorthSouth62DegreesNorth;
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, Display, EnumIter, EnumString};

#[derive(Debug, Clone, PartialEq)]
pub struct CatchLocation {
    pub catch_field: String,
    pub coast_ocean_code: TwelveMileBorder,
    pub main_area_code: Option<u32>,
    pub main_area: Option<String>,
    pub main_area_longitude: Option<f64>,
    pub main_area_latitude: Option<f64>,
    pub location_code: Option<u32>,
    pub location_longitude: Option<f64>,
    pub location_latitude: Option<f64>,
    pub economic_zone_code: Option<String>,
    pub area_grouping: Option<String>,
    pub area_grouping_code: Option<String>,
    pub main_area_fao_code: Option<u32>,
    pub main_area_fao: Option<String>,
    pub north_or_south_of_62_degrees: NorthSouth62DegreesNorth,
}

#[repr(i32)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromPrimitive,
    Copy,
    EnumIter,
    PartialOrd,
    Ord,
    Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum TwelveMileBorder {
    Outside = 0,
    Within = 8,
    Unknown = 9,
}
