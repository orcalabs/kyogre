use std::fmt::Display;

use crate::{CatchLocationId, HaulId, MLModelsInbound, MLModelsOutbound};
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

#[derive(Debug, Copy, Clone, Deserialize, Serialize, strum::Display)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub enum ModelId {
    #[strum(serialize = "fishingSpotPredictor")]
    FishingSpotPredictor = 1,
    #[strum(serialize = "fishingWeightPredictor")]
    FishingWeightPredictor = 2,
    #[strum(serialize = "fishingWeightWeatherPredictor")]
    FishingWeightWeatherPredictor = 3,
}

impl From<ModelId> for i32 {
    fn from(value: ModelId) -> Self {
        value as i32
    }
}

pub enum TrainingOutcome {
    Finished,
    Progress { new_model: Vec<u8> },
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct TrainingHaul {
    pub haul_id: HaulId,
    pub species: SpeciesGroup,
    pub catch_location_id: CatchLocationId,
}

#[derive(Debug)]
pub struct FishingSpotTrainingData {
    pub haul_id: i64,
    pub latitude: f64,
    pub longitude: f64,
    pub weight: f64,
    pub species: SpeciesGroup,
    pub week: i32,
    pub catch_location_id: CatchLocationId,
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
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
}

#[async_trait]
pub trait MLModel: Send + Sync {
    fn id(&self) -> ModelId;
    async fn train(
        &self,
        model: &[u8],
        adapter: &dyn MLModelsOutbound,
    ) -> Result<TrainingOutcome, MLModelError>;
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
    pub model: ModelId,
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
