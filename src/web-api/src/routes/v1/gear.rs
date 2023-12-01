use crate::{error::ApiError, response::Response};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/gear",
    responses(
        (status = 200, description = "all gear types", body = [Gear]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn gear() -> Result<Response<Vec<Gear>>, ApiError> {
    let gear: Vec<Gear> = fiskeridir_rs::Gear::iter().map(Gear::from).collect();
    Ok(Response::new(gear))
}

#[utoipa::path(
    get,
    path = "/gear_groups",
    responses(
        (status = 200, description = "all gear groups", body = [GearGroup]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn gear_groups() -> Result<Response<Vec<GearGroup>>, ApiError> {
    let gear: Vec<GearGroup> = fiskeridir_rs::GearGroup::iter()
        .map(GearGroup::from)
        .collect();
    Ok(Response::new(gear))
}

#[utoipa::path(
    get,
    path = "/gear_main_groups",
    responses(
        (status = 200, description = "all main gear groups", body = [GearMainGroup]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn gear_main_groups() -> Result<Response<Vec<GearMainGroup>>, ApiError> {
    let gear: Vec<GearMainGroup> = fiskeridir_rs::MainGearGroup::iter()
        .map(GearMainGroup::from)
        .collect();
    Ok(Response::new(gear))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Gear {
    pub id: u32,
    pub name: &'static str,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearGroup {
    pub id: u32,
    pub name: &'static str,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearMainGroup {
    pub id: u32,
    pub name: &'static str,
}

impl From<fiskeridir_rs::Gear> for Gear {
    fn from(value: fiskeridir_rs::Gear) -> Self {
        Gear {
            id: value as u32,
            name: value.norwegian_name(),
        }
    }
}

impl From<fiskeridir_rs::GearGroup> for GearGroup {
    fn from(value: fiskeridir_rs::GearGroup) -> Self {
        GearGroup {
            id: value as u32,
            name: value.norwegian_name(),
        }
    }
}

impl From<fiskeridir_rs::MainGearGroup> for GearMainGroup {
    fn from(value: fiskeridir_rs::MainGearGroup) -> Self {
        GearMainGroup {
            id: value as u32,
            name: value.norwegian_name(),
        }
    }
}
