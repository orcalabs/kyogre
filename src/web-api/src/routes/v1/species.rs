use crate::{
    error::ApiError,
    response::Response,
    routes::utils::{from_string, to_string},
    to_streaming_response, Database,
};
use actix_web::{web, HttpResponse};
use fiskeridir_rs::{SpeciesGroup, SpeciesMainGroup};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tracing::{event, Level};
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/species",
    responses(
        (status = 200, description = "all species", body = [Species]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn species<T: Database + 'static>(db: web::Data<T>) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db.species().map_ok(Species::from).map_err(|e| {
            event!(Level::ERROR, "failed to retrieve species: {:?}", e);
            ApiError::InternalServerError
        })
    }
}

#[utoipa::path(
    get,
    path = "/species_groups",
    responses(
        (status = 200, description = "all species groups", body = [SpeciesGroupDetailed]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument]
pub async fn species_groups<T: Database + 'static + 'static>() -> Response<Vec<SpeciesGroupDetailed>>
{
    Response::new(
        fiskeridir_rs::SpeciesGroup::iter()
            .map(SpeciesGroupDetailed::from)
            .collect(),
    )
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
pub async fn species_fiskeridir<T: Database + 'static>(
    db: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db.species_fiskeridir()
            .map_ok(SpeciesFiskeridir::from)
            .map_err(|e| {
                event!(
                    Level::ERROR,
                    "failed to retrieve species_fiskeridir: {:?}",
                    e
                );
                ApiError::InternalServerError
            })
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
pub async fn species_fao<T: Database + 'static>(
    db: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db.species_fao().map_ok(SpeciesFao::from).map_err(|e| {
            event!(Level::ERROR, "failed to retrieve species_fao: {:?}", e);
            ApiError::InternalServerError
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesGroupDetailed {
    #[serde(serialize_with = "to_string", deserialize_with = "from_string")]
    pub id: SpeciesGroup,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SpeciesMainGroupDetailed {
    #[serde(serialize_with = "to_string", deserialize_with = "from_string")]
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
