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
pub struct NewCatchMainArea<'a> {
    #[unnest_insert(field_name = "catch_main_area_id")]
    pub id: i32,
    pub name: Option<&'a str>,
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, UnnestInsert)]
#[unnest_insert(table_name = "area_groupings", conflict = "area_grouping_id")]
pub struct NewAreaGrouping<'a> {
    #[unnest_insert(field_name = "area_grouping_id")]
    pub id: &'a str,
    pub name: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, UnnestInsert)]
#[unnest_insert(
    table_name = "catch_main_area_fao",
    conflict = "catch_main_area_fao_id"
)]
pub struct NewCatchMainAreaFao<'a> {
    #[unnest_insert(field_name = "catch_main_area_fao_id")]
    pub id: i32,
    pub name: Option<&'a str>,
}

impl<'a> NewAreaGrouping<'a> {
    pub fn new(id: &'a str, name: Option<&'a str>) -> Self {
        Self { id, name }
    }

    pub fn from_landing(landing: &'a fiskeridir_rs::Landing) -> Option<Self> {
        landing
            .catch_location
            .area_grouping_code
            .as_ref()
            .map(|id| Self {
                id,
                name: landing.catch_location.area_grouping_code.as_deref(),
            })
    }
}

impl NewCatchArea {
    pub fn from_landing(landing: &fiskeridir_rs::Landing) -> Option<Self> {
        landing.catch_location.location_code.map(|id| Self {
            id: id as i32,
            latitude: landing.catch_location.location_latitude,
            longitude: landing.catch_location.location_longitude,
        })
    }
}

impl<'a> NewCatchMainArea<'a> {
    pub fn from_landing(landing: &'a fiskeridir_rs::Landing) -> Option<Self> {
        landing.catch_location.main_area_code.map(|id| Self {
            id: id as i32,
            name: landing.catch_location.main_area.as_deref(),
            latitude: landing.catch_location.main_area_latitude,
            longitude: landing.catch_location.main_area_longitude,
        })
    }
}

impl<'a> NewCatchMainAreaFao<'a> {
    pub fn from_landing(landing: &'a fiskeridir_rs::Landing) -> Option<Self> {
        landing.catch_location.main_area_fao_code.map(|id| Self {
            id: id as i32,
            name: landing.catch_location.main_area_fao.as_deref(),
        })
    }
}
