use std::fmt::Display;

use crate::{CatchLocationId, MLModelsInbound, MLModelsOutbound};
use async_trait::async_trait;
use error_stack::{Context, Result};
use fiskeridir_rs::SpeciesGroup;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum MLModelError {
    StoreOutput,
    Python,
    DataPreparation,
}

impl Display for MLModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MLModelError::DataPreparation => f.write_str("failed to prepare training data"),
            MLModelError::StoreOutput => f.write_str("failed to store output predictions"),
            MLModelError::Python => f.write_str("a python related error occurred"),
        }
    }
}

impl Context for MLModelError {}

#[derive(Debug, Copy, Clone)]
pub enum ModelId {
    FishingSpotPredictor = 1,
    FishingWeightPredictor = 2,
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelId::FishingSpotPredictor => f.write_str("fishing_spot_predictor"),
            ModelId::FishingWeightPredictor => f.write_str("fishing_weight_predictor"),
        }
    }
}

#[derive(Debug)]
pub struct FishingSpotTrainingData {
    pub haul_id: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub weight: f64,
    pub species: SpeciesGroup,
    pub week: i32,
}

#[derive(Debug)]
pub struct WeightPredictorTrainingData {
    pub haul_id: i64,
    pub weight: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub catch_location: CatchLocationId,
    pub species: SpeciesGroup,
    pub week: i32,
}

#[async_trait]
pub trait MLModel: Send + Sync {
    fn id(&self) -> ModelId;
    async fn train(
        &self,
        model: Vec<u8>,
        adapter: &dyn MLModelsOutbound,
    ) -> Result<Vec<u8>, MLModelError>;
    async fn predict(
        &self,
        model: &[u8],
        adapter: &dyn MLModelsInbound,
    ) -> Result<(), MLModelError>;
}

#[derive(Debug, Clone)]
pub struct NewFishingSpotPrediction {
    pub latitude: f64,
    pub longitude: f64,
    pub species: SpeciesGroup,
    pub week: u32,
    pub year: u32,
}

#[derive(Debug, Clone)]
pub struct NewFishingWeightPrediction {
    pub catch_location_id: CatchLocationId,
    pub weight: f64,
    pub species: SpeciesGroup,
    pub week: u32,
    pub year: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FishingSpotPrediction {
    pub latitude: f64,
    pub longitude: f64,
    pub species: i32,
    pub week: i32,
    pub year: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FishingWeightPrediction {
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub catch_location_id: CatchLocationId,
    pub weight: f64,
    #[cfg_attr(feature = "utoipa", schema(value_type = i32))]
    pub species_group_id: SpeciesGroup,
    pub week: u32,
    pub year: u32,
}
