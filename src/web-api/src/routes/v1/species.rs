use actix_web::web;
use fiskeridir_rs::{Condition, Quality, SpeciesGroup, SpeciesMainGroup};
use futures::TryStreamExt;
use kyogre_core::SpeciesFiskeridir;
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use strum::IntoEnumIterator;

use crate::{
    Database,
    response::{Response, StreamResponse},
    stream_response,
};

#[oasgen(skip(db), tags("Species"))]
#[tracing::instrument(skip(db))]
pub async fn species<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<Species> {
    stream_response! {
        db.species().map_ok(Species::from)
    }
}

#[oasgen(tags("Species"))]
#[tracing::instrument]
pub async fn species_groups() -> Response<Vec<SpeciesGroupDetailed>> {
    Response::new(
        fiskeridir_rs::SpeciesGroup::iter()
            .map(SpeciesGroupDetailed::from)
            .collect(),
    )
}

#[oasgen(tags("Species"))]
#[tracing::instrument]
pub async fn species_main_groups() -> Response<Vec<SpeciesMainGroupDetailed>> {
    Response::new(
        fiskeridir_rs::SpeciesMainGroup::iter()
            .map(SpeciesMainGroupDetailed::from)
            .collect(),
    )
}

#[oasgen(skip(db), tags("Species"))]
#[tracing::instrument(skip(db))]
pub async fn species_fiskeridir<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<SpeciesFiskeridir> {
    stream_response! {
        db.species_fiskeridir()
    }
}

#[oasgen(skip(db), tags("Species"))]
#[tracing::instrument(skip(db))]
pub async fn species_fao<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<SpeciesFao> {
    stream_response! {
        db.species_fao().map_ok(SpeciesFao::from)
    }
}

#[oasgen(tags("Species"))]
#[tracing::instrument]
pub async fn conditions() -> Response<Vec<ConditionDetailed>> {
    Response::new(fiskeridir_rs::Condition::iter().map(From::from).collect())
}

#[oasgen(tags("Species"))]
#[tracing::instrument]
pub async fn qualities() -> Response<Vec<QualityDetailed>> {
    Response::new(fiskeridir_rs::Quality::iter().map(From::from).collect())
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: SpeciesGroup,
    pub name: String,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesMainGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: SpeciesMainGroup,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConditionDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Condition,
    pub name: &'static str,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct QualityDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Quality,
    pub name: &'static str,
}

impl From<kyogre_core::Species> for Species {
    fn from(value: kyogre_core::Species) -> Self {
        Species {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<fiskeridir_rs::SpeciesGroup> for SpeciesGroupDetailed {
    fn from(value: fiskeridir_rs::SpeciesGroup) -> Self {
        SpeciesGroupDetailed {
            name: value.norwegian_name().to_owned(),
            id: value,
        }
    }
}

impl From<fiskeridir_rs::SpeciesMainGroup> for SpeciesMainGroupDetailed {
    fn from(value: fiskeridir_rs::SpeciesMainGroup) -> Self {
        SpeciesMainGroupDetailed {
            name: value.norwegian_name().to_owned(),
            id: value,
        }
    }
}

impl From<kyogre_core::SpeciesFao> for SpeciesFao {
    fn from(value: kyogre_core::SpeciesFao) -> Self {
        SpeciesFao {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<Condition> for ConditionDetailed {
    fn from(value: Condition) -> Self {
        Self {
            id: value,
            name: value.name(),
        }
    }
}

impl From<Quality> for QualityDetailed {
    fn from(value: Quality) -> Self {
        Self {
            id: value,
            name: value.norwegian_name(),
        }
    }
}
