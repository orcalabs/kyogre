use crate::{error::ApiError, response::Response};
use fiskeridir_rs::{Gear, GearGroup, MainGearGroup};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use strum::IntoEnumIterator;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/gear",
    responses(
        (status = 200, description = "all gear types", body = [GearDetailed]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn gear() -> Result<Response<Vec<GearDetailed>>, ApiError> {
    let gear: Vec<GearDetailed> = Gear::iter().map(GearDetailed::from).collect();
    Ok(Response::new(gear))
}

#[utoipa::path(
    get,
    path = "/gear_groups",
    responses(
        (status = 200, description = "all gear groups", body = [GearGroupDetailed]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn gear_groups() -> Result<Response<Vec<GearGroupDetailed>>, ApiError> {
    let gear: Vec<GearGroupDetailed> = GearGroup::iter().map(GearGroupDetailed::from).collect();
    Ok(Response::new(gear))
}

#[utoipa::path(
    get,
    path = "/gear_main_groups",
    responses(
        (status = 200, description = "all main gear groups", body = [GearMainGroupDetailed]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn gear_main_groups() -> Result<Response<Vec<GearMainGroupDetailed>>, ApiError> {
    let gear: Vec<GearMainGroupDetailed> = MainGearGroup::iter()
        .map(GearMainGroupDetailed::from)
        .collect();
    Ok(Response::new(gear))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Gear,
    pub name: &'static str,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: GearGroup,
    pub name: &'static str,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearMainGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: MainGearGroup,
    pub name: &'static str,
}

impl From<Gear> for GearDetailed {
    fn from(value: Gear) -> Self {
        GearDetailed {
            id: value,
            name: value.norwegian_name(),
        }
    }
}

impl From<GearGroup> for GearGroupDetailed {
    fn from(value: GearGroup) -> Self {
        GearGroupDetailed {
            id: value,
            name: value.norwegian_name(),
        }
    }
}

impl From<MainGearGroup> for GearMainGroupDetailed {
    fn from(value: MainGearGroup) -> Self {
        GearMainGroupDetailed {
            id: value,
            name: value.norwegian_name(),
        }
    }
}
