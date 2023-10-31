use crate::{error::ApiError, response::Response};
use crate::{to_streaming_response, Database};
use actix_web::HttpResponse;
use actix_web::{web, web::Path};
use chrono::Datelike;
use chrono::Utc;
use fiskeridir_rs::SpeciesGroup;
use futures::TryStreamExt;
use kyogre_core::FishingSpotPrediction;
use num_traits::FromPrimitive;
use serde::Deserialize;
use tracing::{event, Level};
use utoipa::IntoParams;

pub const MAX_FISHING_WEIGHT_PREDICTIONS: u32 = 20;
pub const DEFAULT_FISHING_WEIGHT_PREDICTIONS: u32 = 5;

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FishingSpotPredictionParams {
    pub week: Option<u32>,
}

#[derive(Default, Debug, Clone, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FishingWeightPredictionParams {
    pub week: Option<u32>,
    pub limit: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/fishing_spot_predictions/{species_group_id}",
    params(FishingSpotPredictionParams),
    responses(
        (status = 200, description = "fishing spot predictions for the requested filter", body = FishingSpotPrediction),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_spot_predictions<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<FishingSpotPredictionParams>,
    species_group_id: Path<u32>,
) -> Result<Response<Option<FishingSpotPrediction>>, ApiError> {
    let species_group_id = species_group_id.into_inner();
    let species_group_id = SpeciesGroup::from_u32(species_group_id)
        .ok_or(ApiError::InvalidSpeciesGroupId(species_group_id))?;

    let week = params.week.unwrap_or_else(|| Utc::now().iso_week().week());

    match db.fishing_spot_prediction(species_group_id, week).await {
        Ok(v) => Ok(Response::new(v)),
        Err(e) => {
            event!(
                Level::ERROR,
                "failed to retrieve fishing spot predictions: {:?}",
                e
            );
            Err(ApiError::InternalServerError)
        }
    }
}

#[utoipa::path(
    get,
    path = "/fishing_spot_predictions",
    responses(
        (status = 200, description = "all fishing spot predictions", body = [FishingSpotPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn all_fishing_spot_predictions<T: Database + 'static>(
    db: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db
         .all_fishing_spot_predictions()
         .map_err(|e| {
            event!(
                Level::ERROR,
                "failed to retrieve fishing spot predictions: {:?}",
                e
            );
            ApiError::InternalServerError
         })
    }
}

#[utoipa::path(
    get,
    path = "/fishing_weight_predictions/{species_group_id}",
    params(FishingWeightPredictionParams),
    responses(
        (status = 200, description = "fishing weight predictions for the requested filter", body = FishingWeightPrediction),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_weight_predictions<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<FishingWeightPredictionParams>,
    species_group_id: Path<u32>,
) -> Result<HttpResponse, ApiError> {
    let species_group_id = species_group_id.into_inner();
    let species_group_id = SpeciesGroup::from_u32(species_group_id)
        .ok_or(ApiError::InvalidSpeciesGroupId(species_group_id))?;

    let week = params.week.unwrap_or_else(|| Utc::now().iso_week().week());
    let mut limit = params.limit.unwrap_or(DEFAULT_FISHING_WEIGHT_PREDICTIONS);

    if limit > MAX_FISHING_WEIGHT_PREDICTIONS {
        limit = DEFAULT_FISHING_WEIGHT_PREDICTIONS;
    }

    to_streaming_response! {
        db
         .fishing_weight_predictions(species_group_id, week, limit)
         .map_err(|e| {
            event!(
                Level::ERROR,
                "failed to retrieve fishing weight predictions: {:?}",
                e
            );
            ApiError::InternalServerError
         })
    }
}

#[utoipa::path(
    get,
    path = "/fishing_weight_predictions",
    responses(
        (status = 200, description = "all fishing weight predictions", body = [FishingWeightPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn all_fishing_weight_predictions<T: Database + 'static>(
    db: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db
         .all_fishing_weight_predictions()
         .map_err(|e| {
            event!(
                Level::ERROR,
                "failed to retrieve all fishing weight predictions: {:?}",
                e
            );
            ApiError::InternalServerError
         })
    }
}
