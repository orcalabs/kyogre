use actix_web::web;
use fiskeridir_rs::{SpeciesGroup, SpeciesMainGroup};
use futures::TryStreamExt;
use kyogre_core::ML_SPECIES_GROUPS;
use oasgen::{OaSchema, oasgen};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{DisplayFromStr, serde_as};
use strum::IntoEnumIterator;

use crate::{
    Database,
    response::{Response, StreamResponse},
    stream_response,
};

#[derive(Default, Debug, Clone, Deserialize, Serialize, OaSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesGroupParams {
    pub has_ml_models: Option<bool>,
}

#[oasgen(skip(db), tags("Species"))]
#[tracing::instrument(skip(db))]
pub async fn species<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<Species> {
    stream_response! {
        db.species().map_ok(Species::from)
    }
}

#[oasgen(skip(db), tags("Species"))]
#[tracing::instrument]
pub async fn species_groups<T: Database + 'static + 'static>(
    params: Query<SpeciesGroupParams>,
) -> Response<Vec<SpeciesGroupDetailed>> {
    if params.into_inner().has_ml_models.unwrap_or(false) {
        Response::new(
            ML_SPECIES_GROUPS
                .iter()
                .map(|v| SpeciesGroupDetailed::from(*v))
                .collect(),
        )
    } else {
        Response::new(
            fiskeridir_rs::SpeciesGroup::iter()
                .map(SpeciesGroupDetailed::from)
                .collect(),
        )
    }
}

#[oasgen(skip(db), tags("Species"))]
#[tracing::instrument]
pub async fn species_main_groups<T: Database + 'static>() -> Response<Vec<SpeciesMainGroupDetailed>>
{
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
        db.species_fiskeridir().map_ok(SpeciesFiskeridir::from)
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
pub struct SpeciesFiskeridir {
    pub id: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, OaSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesFao {
    pub id: String,
    pub name: Option<String>,
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

impl From<kyogre_core::SpeciesFiskeridir> for SpeciesFiskeridir {
    fn from(value: kyogre_core::SpeciesFiskeridir) -> Self {
        SpeciesFiskeridir {
            id: value.id,
            name: value.name,
        }
    }
}
