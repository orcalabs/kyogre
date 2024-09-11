use crate::error::Result;
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone, PartialEq, UnnestInsert)]
#[unnest_insert(
    table_name = "catch_areas",
    conflict = "catch_area_id",
    update_coalesce_all
)]
pub struct NewCatchArea {
    #[unnest_insert(field_name = "catch_area_id")]
    pub id: i32,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, UnnestInsert)]
#[unnest_insert(
    table_name = "catch_main_areas",
    conflict = "catch_main_area_id",
    update_coalesce_all
)]
pub struct NewCatchMainArea {
    #[unnest_insert(field_name = "catch_main_area_id")]
    pub id: i32,
    pub name: Option<String>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "area_groupings", conflict = "area_grouping_id")]
pub struct NewAreaGrouping {
    #[unnest_insert(field_name = "area_grouping_id")]
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, UnnestInsert)]
#[unnest_insert(
    table_name = "catch_main_area_fao",
    conflict = "catch_main_area_fao_id"
)]
pub struct NewCatchMainAreaFao {
    #[unnest_insert(field_name = "catch_main_area_fao_id")]
    pub id: i32,
    pub name: Option<String>,
}

impl NewAreaGrouping {
    pub fn new(id: String, name: Option<String>) -> Self {
        Self { id, name }
    }

    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Option<NewAreaGrouping> {
        landing
            .catch_location
            .area_grouping_code
            .as_ref()
            .map(|id| NewAreaGrouping {
                id: id.clone().into_inner(),
                name: landing
                    .catch_location
                    .area_grouping_code
                    .clone()
                    .map(|v| v.into_inner()),
            })
    }
}

impl NewCatchArea {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Result<Option<NewCatchArea>> {
        landing
            .catch_location
            .location_code
            .map(|id| {
                Ok(NewCatchArea {
                    id: id as i32,
                    latitude: landing.catch_location.location_latitude,
                    longitude: landing.catch_location.location_longitude,
                })
            })
            .transpose()
    }
}
impl NewCatchMainArea {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Result<Option<Self>> {
        landing
            .catch_location
            .main_area_code
            .map(|id| {
                Ok(Self {
                    id: id as i32,
                    name: landing
                        .catch_location
                        .main_area
                        .clone()
                        .map(|v| v.into_inner()),
                    latitude: landing.catch_location.main_area_latitude,
                    longitude: landing.catch_location.main_area_longitude,
                })
            })
            .transpose()
    }
}

impl NewCatchMainAreaFao {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Option<NewCatchMainAreaFao> {
        landing
            .catch_location
            .main_area_fao_code
            .map(|id| NewCatchMainAreaFao {
                id: id as i32,
                name: landing
                    .catch_location
                    .main_area_fao
                    .clone()
                    .map(|v| v.into_inner()),
            })
    }
}
