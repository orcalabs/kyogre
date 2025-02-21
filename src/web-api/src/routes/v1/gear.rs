use fiskeridir_rs::{Gear, GearGroup, MainGearGroup};
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use strum::IntoEnumIterator;

use crate::{error::Result, response::Response};

#[oasgen(skip(db), tags("Gear"))]
#[tracing::instrument]
pub async fn gear() -> Result<Response<Vec<GearDetailed>>> {
    let gear: Vec<GearDetailed> = Gear::iter().map(GearDetailed::from).collect();
    Ok(Response::new(gear))
}

#[oasgen(skip(db), tags("Gear"))]
#[tracing::instrument]
pub async fn gear_groups() -> Result<Response<Vec<GearGroupDetailed>>> {
    let gear: Vec<GearGroupDetailed> = GearGroup::iter().map(GearGroupDetailed::from).collect();
    Ok(Response::new(gear))
}

#[oasgen(skip(db), tags("Gear"))]
#[tracing::instrument]
pub async fn gear_main_groups() -> Result<Response<Vec<GearMainGroupDetailed>>> {
    let gear: Vec<GearMainGroupDetailed> = MainGearGroup::iter()
        .map(GearMainGroupDetailed::from)
        .collect();
    Ok(Response::new(gear))
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Gear,
    pub name: &'static str,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GearGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: GearGroup,
    pub name: &'static str,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
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
