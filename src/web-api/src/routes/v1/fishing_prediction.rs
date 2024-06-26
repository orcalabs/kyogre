use crate::{error::ApiError, response::Response};
use crate::{to_streaming_response, Database};
use actix_web::HttpResponse;
use actix_web::{web, web::Path};
use chrono::NaiveDate;
use chrono::Utc;
use fiskeridir_rs::SpeciesGroup;
use futures::TryStreamExt;
use kyogre_core::{CatchLocationId, ModelId};
use serde::{Deserialize, Serialize};
use serde_qs::actix::QsQuery as Query;
use serde_with::{serde_as, DisplayFromStr};
use tracing::{event, Level};
use utoipa::{IntoParams, ToSchema};

pub const MAX_FISHING_WEIGHT_PREDICTIONS: u32 = 20;
pub const DEFAULT_FISHING_WEIGHT_PREDICTIONS: u32 = 5;

#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FishingSpotPredictionParams {
    pub date: Option<NaiveDate>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct FishingWeightPredictionParams {
    pub date: Option<NaiveDate>,
    pub limit: Option<u32>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, IntoParams)]
pub struct FishingPredictionsPath {
    #[serde_as(as = "DisplayFromStr")]
    pub model_id: ModelId,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, IntoParams)]
pub struct AllFishingPredictionsPath {
    #[serde_as(as = "DisplayFromStr")]
    pub model_id: ModelId,
}

#[utoipa::path(
    get,
    path = "/fishing_spot_predictions/{model_id}/{species_group_id}",
    params(
        FishingSpotPredictionParams,
        FishingPredictionsPath,
    ),
    responses(
        (status = 200, description = "fishing spot predictions for the requested filter", body = FishingSpotPrediction),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn fishing_spot_predictions<T: Database + 'static>(
    db: web::Data<T>,
    params: Query<FishingSpotPredictionParams>,
    path_params: Path<FishingPredictionsPath>,
) -> Result<Response<Option<FishingSpotPrediction>>, ApiError> {
    let date = params.date.unwrap_or_else(|| Utc::now().date_naive());

    match db
        .fishing_spot_prediction(path_params.model_id, path_params.species_group_id, date)
        .await
    {
        Ok(v) => Ok(Response::new(v.map(FishingSpotPrediction::from))),
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
    params(AllFishingPredictionsPath),
    responses(
        (status = 200, description = "all fishing spot predictions", body = [FishingSpotPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn all_fishing_spot_predictions<T: Database + 'static>(
    db: web::Data<T>,
    path: Path<AllFishingPredictionsPath>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db
         .all_fishing_spot_predictions(path.model_id)
         .map_ok(FishingSpotPrediction::from)
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
        FishingPredictionsPath,
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
    path_params: Path<FishingPredictionsPath>,
) -> Result<HttpResponse, ApiError> {
    let date = params.date.unwrap_or_else(|| Utc::now().date_naive());
    let mut limit = params.limit.unwrap_or(DEFAULT_FISHING_WEIGHT_PREDICTIONS);

    if limit > MAX_FISHING_WEIGHT_PREDICTIONS {
        limit = DEFAULT_FISHING_WEIGHT_PREDICTIONS;
    }

    to_streaming_response! {
        db
         .fishing_weight_predictions(path_params.model_id, path_params.species_group_id, date, limit)
         .map_ok(FishingWeightPrediction::from)
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
    params(AllFishingPredictionsPath),
    responses(
        (status = 200, description = "all fishing weight predictions", body = [FishingWeightPrediction]),
        (status = 500, description = "an internal error occured", body = ErrorResponse),
    )
)]
#[tracing::instrument(skip(db))]
pub async fn all_fishing_weight_predictions<T: Database + 'static>(
    db: web::Data<T>,
    path: Path<AllFishingPredictionsPath>,
) -> Result<HttpResponse, ApiError> {
    to_streaming_response! {
        db
         .all_fishing_weight_predictions(path.model_id)
         .map_ok(FishingWeightPrediction::from)
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

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingSpotPrediction {
    pub latitude: f64,
    pub longitude: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
    pub date: NaiveDate,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FishingWeightPrediction {
    #[schema(value_type = String)]
    pub catch_location_id: CatchLocationId,
    pub weight: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
    pub date: NaiveDate,
}

impl From<kyogre_core::FishingSpotPrediction> for FishingSpotPrediction {
    fn from(v: kyogre_core::FishingSpotPrediction) -> Self {
        Self {
            latitude: v.latitude,
            longitude: v.longitude,
            species_group_id: v.species_group_id,
            date: v.date,
        }
    }
}

impl From<kyogre_core::FishingWeightPrediction> for FishingWeightPrediction {
    fn from(v: kyogre_core::FishingWeightPrediction) -> Self {
        Self {
            catch_location_id: v.catch_location_id,
            weight: v.weight,
            species_group_id: v.species_group_id,
            date: v.date,
        }
    }
}
