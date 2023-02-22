use crate::{error::ApiError, response::Response, Database};
use actix_web::web;
use serde::{Deserialize, Serialize};
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
pub async fn species<T: Database>(db: web::Data<T>) -> Result<Response<Vec<Species>>, ApiError> {
    let species = db
        .species()
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve species: {:?}", e);
            ApiError::InternalServerError
        })?
        .into_iter()
        .map(Species::from)
        .collect();

    Ok(Response::new(species))
}

#[utoipa::path(
    get,
    path = "/species_groups",
    responses(
        (status = 200, description = "all species groups", body = [SpeciesGroup]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn species_groups<T: Database>(
    db: web::Data<T>,
) -> Result<Response<Vec<SpeciesGroup>>, ApiError> {
    let species = db
        .species_groups()
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve species_groups: {:?}", e);
            ApiError::InternalServerError
        })?
        .into_iter()
        .map(SpeciesGroup::from)
        .collect();

    Ok(Response::new(species))
}

#[utoipa::path(
    get,
    path = "/species_main_groups",
    responses(
        (status = 200, description = "all species main groups", body = [SpeciesMainGroup]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn species_main_groups<T: Database>(
    db: web::Data<T>,
) -> Result<Response<Vec<SpeciesMainGroup>>, ApiError> {
    let species = db
        .species_main_groups()
        .await
        .map_err(|e| {
            event!(
                Level::ERROR,
                "failed to retrieve species_main_groups: {:?}",
                e
            );
            ApiError::InternalServerError
        })?
        .into_iter()
        .map(SpeciesMainGroup::from)
        .collect();

    Ok(Response::new(species))
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
pub async fn species_fiskeridir<T: Database>(
    db: web::Data<T>,
) -> Result<Response<Vec<SpeciesFiskeridir>>, ApiError> {
    let species = db
        .species_fiskeridir()
        .await
        .map_err(|e| {
            event!(
                Level::ERROR,
                "failed to retrieve species_fiskeridir: {:?}",
                e
            );
            ApiError::InternalServerError
        })?
        .into_iter()
        .map(SpeciesFiskeridir::from)
        .collect();

    Ok(Response::new(species))
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
pub async fn species_fao<T: Database>(
    db: web::Data<T>,
) -> Result<Response<Vec<SpeciesFao>>, ApiError> {
    let species = db
        .species_fao()
        .await
        .map_err(|e| {
            event!(Level::ERROR, "failed to retrieve species_fao: {:?}", e);
            ApiError::InternalServerError
        })?
        .into_iter()
        .map(SpeciesFao::from)
        .collect();

    Ok(Response::new(species))
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
pub struct Species {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
pub struct SpeciesGroup {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
pub struct SpeciesMainGroup {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
pub struct SpeciesFiskeridir {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, Ord, PartialOrd, PartialEq, Eq)]
pub struct SpeciesFao {
    pub id: String,
    pub name: String,
}

impl From<kyogre_core::Species> for Species {
    fn from(value: kyogre_core::Species) -> Self {
        Species {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<kyogre_core::SpeciesGroup> for SpeciesGroup {
    fn from(value: kyogre_core::SpeciesGroup) -> Self {
        SpeciesGroup {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<kyogre_core::SpeciesMainGroup> for SpeciesMainGroup {
    fn from(value: kyogre_core::SpeciesMainGroup) -> Self {
        SpeciesMainGroup {
            id: value.id,
            name: value.name,
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
