use bigdecimal::BigDecimal;
use error_stack::{Report, ResultExt};
use unnest_insert::UnnestInsert;

use crate::{error::PostgresError, queries::opt_float_to_decimal};

#[derive(Debug, Clone, PartialEq, UnnestInsert)]
#[unnest_insert(
    table_name = "catch_areas",
    conflict = "catch_area_id",
    update_coalesce_all
)]
pub struct NewCatchArea {
    #[unnest_insert(field_name = "catch_area_id")]
    pub id: i32,
    pub longitude: Option<BigDecimal>,
    pub latitude: Option<BigDecimal>,
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
    pub longitude: Option<BigDecimal>,
    pub latitude: Option<BigDecimal>,
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
                id: id.clone(),
                name: landing.catch_location.area_grouping_code.clone(),
            })
    }
}

impl NewCatchArea {
    pub fn from_landing(
        landing: &fiskeridir_rs::Landing,
    ) -> Result<Option<NewCatchArea>, Report<PostgresError>> {
        landing
            .catch_location
            .location_code
            .map(|id| {
                Ok(NewCatchArea {
                    id: id as i32,
                    latitude: opt_float_to_decimal(landing.catch_location.location_latitude)
                        .change_context(PostgresError::DataConversion)?,
                    longitude: opt_float_to_decimal(landing.catch_location.location_longitude)
                        .change_context(PostgresError::DataConversion)?,
                })
            })
            .transpose()
    }
}
impl NewCatchMainArea {
    pub fn from_landing(
        landing: &fiskeridir_rs::Landing,
    ) -> Result<Option<Self>, Report<PostgresError>> {
        landing
            .catch_location
            .main_area_code
            .map(|id| {
                Ok(Self {
                    id: id as i32,
                    name: landing.catch_location.main_area.clone(),
                    latitude: opt_float_to_decimal(landing.catch_location.main_area_latitude)
                        .change_context(PostgresError::DataConversion)?,
                    longitude: opt_float_to_decimal(landing.catch_location.main_area_longitude)
                        .change_context(PostgresError::DataConversion)?,
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
                name: landing.catch_location.main_area_fao.clone(),
            })
    }
}
