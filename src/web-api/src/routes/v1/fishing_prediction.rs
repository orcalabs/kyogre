use crate::{error::ApiError, response::Response};
use crate::{to_streaming_response, Database};
use actix_web::HttpResponse;
use actix_web::{web, web::Path};
use chrono::Datelike;
use chrono::Utc;
use fiskeridir_rs::SpeciesGroup;
use futures::TryStreamExt;
use kyogre_core::{FishingSpotPrediction, ModelId};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Copy, Clone, Deserialize, Serialize, strum::Display, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum FishingSpotPredictor {
    #[strum(serialize = "base")]
    Base,
    #[strum(serialize = "weather")]
    Weather,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize, strum::Display, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum FishingWeightPredictor {
    #[strum(serialize = "base")]
    Base,
    #[strum(serialize = "weather")]
    Weather,
}

impl From<FishingWeightPredictor> for ModelId {
    fn from(value: FishingWeightPredictor) -> Self {
        match value {
            FishingWeightPredictor::Base => ModelId::FishingWeightPredictor,
            FishingWeightPredictor::Weather => ModelId::FishingWeightWeatherPredictor,
        }
    }
}

impl From<FishingSpotPredictor> for ModelId {
    fn from(value: FishingSpotPredictor) -> Self {
        match value {
            FishingSpotPredictor::Base => ModelId::FishingSpotPredictor,
            FishingSpotPredictor::Weather => ModelId::FishingSpotWeatherPredictor,
        }
    }
}

#[utoipa::path(
    get,
    path = "/fishing_spot_predictions/{model_id}/{species_group_id}",
    params(
        FishingSpotPredictionParams,
        ("model_id" = FishingSpotPredictor, Path, description = "what model data to retrieve")
    ),
    responses(
        (status = 200, description = "fishing spot predictions for the requested filter", body = FishingSpotPrediction),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_spot_predictions<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<FishingSpotPredictionParams>,
    path_params: Path<(FishingSpotPredictor, u32)>,
) -> Result<Response<Option<FishingSpotPrediction>>, ApiError> {
    let path_params = path_params.into_inner();
    let species_group_id = SpeciesGroup::from_u32(path_params.1)
        .ok_or(ApiError::InvalidSpeciesGroupId(path_params.1))?;

    let week = params.week.unwrap_or_else(|| Utc::now().iso_week().week());

    let model = ModelId::from(path_params.0);

    match db
        .fishing_spot_prediction(model, species_group_id, week)
        .await
    {
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
    path = "/fishing_spot_predictions/{model_id}",
    params(
        ("model_id" = FishingSpotPredictor, Path, description = "what model data to retrieve")
    ),
    responses(
        (status = 200, description = "all fishing spot predictions", body = [FishingSpotPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn all_fishing_spot_predictions<T: Database + 'static>(
    db: web::Data<T>,
    model_id: Path<FishingSpotPredictor>,
) -> Result<HttpResponse, ApiError> {
    let model = ModelId::from(model_id.into_inner());
    to_streaming_response! {
        db
         .all_fishing_spot_predictions(model)
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
    path = "/fishing_weight_predictions/{model_id}/{species_group_id}",
    params(
        FishingWeightPredictionParams,
        ("model_id" = FishingWeightPredictor, Path, description = "what model data to retrieve"),
        ("species_group_id" = u32, Path, description = "what species group to retrieve a prediction for")
    ),
    responses(
        (status = 200, description = "fishing weight predictions for the requested filter", body = [FishingWeightPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_weight_predictions<T: Database + 'static>(
    db: web::Data<T>,
    params: web::Query<FishingWeightPredictionParams>,
    path_params: Path<(FishingWeightPredictor, u32)>,
) -> Result<HttpResponse, ApiError> {
    let path_args = path_params.into_inner();
    let species_group_id =
        SpeciesGroup::from_u32(path_args.1).ok_or(ApiError::InvalidSpeciesGroupId(path_args.1))?;

    let week = params.week.unwrap_or_else(|| Utc::now().iso_week().week());
    let mut limit = params.limit.unwrap_or(DEFAULT_FISHING_WEIGHT_PREDICTIONS);

    if limit > MAX_FISHING_WEIGHT_PREDICTIONS {
        limit = DEFAULT_FISHING_WEIGHT_PREDICTIONS;
    }

    let model = ModelId::from(path_args.0);

    to_streaming_response! {
        db
         .fishing_weight_predictions(model, species_group_id, week, limit)
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
    path = "/fishing_weight_predictions/{model_id}",
    params(
        ("model_id" = FishingWeightPredictor, Path, description = "what model data to retrieve")
    ),
    responses(
        (status = 200, description = "all fishing weight predictions", body = [FishingWeightPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn all_fishing_weight_predictions<T: Database + 'static>(
    db: web::Data<T>,
    model_id: Path<FishingWeightPredictor>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db
         .all_fishing_weight_predictions(ModelId::from(model_id.into_inner()))
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
