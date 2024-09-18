use actix_web::web;
use fiskeridir_rs::{SpeciesGroup, SpeciesMainGroup};
use futures::TryStreamExt;
use kyogre_core::ML_SPECIES_GROUPS;
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use strum::IntoEnumIterator;
use utoipa::{IntoParams, ToSchema};

use crate::{
    response::{Response, StreamResponse},
    stream_response, Database,
};

#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesGroupParams {
    pub has_ml_models: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/species",
    responses(
        (status = 200, description = "all species", body = [Species]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn species<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<Species> {
    stream_response! {
        db.species().map_ok(Species::from)
    }
}

#[utoipa::path(
    get,
    path = "/species_groups",
    params(SpeciesGroupParams),
    responses(
        (status = 200, description = "all species groups", body = [SpeciesGroupDetailed]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
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

#[utoipa::path(
    get,
    path = "/species_main_groups",
    responses(
        (status = 200, description = "all species main groups", body = [SpeciesMainGroupDetailed]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn species_main_groups<T: Database + 'static>() -> Response<Vec<SpeciesMainGroupDetailed>>
{
    Response::new(
        fiskeridir_rs::SpeciesMainGroup::iter()
            .map(SpeciesMainGroupDetailed::from)
            .collect(),
    )
}

#[utoipa::path(
    get,
    path = "/species_fiskeridir",
    responses(
        (status = 200, description = "all Fiskeriderktoratet species", body = [SpeciesFiskeridir]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn species_fiskeridir<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<SpeciesFiskeridir> {
    stream_response! {
        db.species_fiskeridir().map_ok(SpeciesFiskeridir::from)
    }
}

#[utoipa::path(
    get,
    path = "/species_fao",
    responses(
        (status = 200, description = "all fao species", body = [SpeciesFao]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn species_fao<T: Database + Send + Sync + 'static>(
    db: web::Data<T>,
) -> StreamResponse<SpeciesFao> {
    stream_response! {
        db.species_fao().map_ok(SpeciesFao::from)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: SpeciesGroup,
    pub name: String,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesMainGroupDetailed {
    #[serde_as(as = "DisplayFromStr")]
    pub id: SpeciesMainGroup,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesFiskeridir {
    pub id: u32,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
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
