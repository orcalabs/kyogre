use crate::{CatchLocationId, HaulId, MLModelsInbound, MLModelsOutbound};
use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use error_stack::{Context, Result};
use fiskeridir_rs::SpeciesGroup;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Display};
use strum::EnumIter;

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

#[derive(Debug, Copy, Clone, Deserialize, Serialize, strum::Display, EnumIter)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum ModelId {
    Spot = 1,
    Weight = 2,
    WeightWeather = 3,
    SpotWeather = 4,
}

impl From<ModelId> for i32 {
    fn from(value: ModelId) -> Self {
        value as i32
    }
}

pub struct SpeciesGroupWeek {
    pub species: SpeciesGroup,
    pub weeks: HashSet<u32>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum TrainingMode {
    Single,
    Batches(u32),
    // Run in single mode without storing the model in the db
    // and hauls in the training log.
    // Intended for faster runs locally.
    Local,
}

impl TrainingMode {
    pub fn batch_size(&self) -> Option<u32> {
        match self {
            TrainingMode::Local | TrainingMode::Single => None,
            TrainingMode::Batches(b) => Some(*b),
        }
    }
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
    pub species: SpeciesGroup,
    pub date: NaiveDate,
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
    pub date: NaiveDate,
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
    pub model: ModelId,
    pub date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct NewFishingWeightPrediction {
    pub model: ModelId,
    pub catch_location_id: CatchLocationId,
    pub weight: f64,
    pub species: SpeciesGroup,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct FishingSpotPrediction {
    pub latitude: f64,
    pub longitude: f64,
    #[cfg_attr(feature = "utoipa", schema(value_type = i32))]
    pub species_group_id: SpeciesGroup,
    pub date: NaiveDate,
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
    pub date: NaiveDate,
}

#[derive(Clone)]
pub enum PredictionRange {
    CurrentYear,
    DaysFromStartOfYear(u32),
    PriorCurrentAndNextWeek,
}

impl PredictionRange {
    pub fn prediction_dates(&self) -> Vec<NaiveDate> {
        let now = Utc::now();
        let current_day = now.ordinal();
        let current_year = now.year() as u32;

        match self {
            PredictionRange::CurrentYear => {
                let mut targets = Vec::with_capacity(current_day as usize);

                let mut current = NaiveDate::from_ymd_opt(current_year as i32, 1, 1).unwrap();
                let end = now.date_naive();

                while current <= end {
                    targets.push(current);
                    current = current.succ_opt().unwrap();
                }

                targets
            }
            PredictionRange::DaysFromStartOfYear(max_day) => {
                let mut targets = Vec::with_capacity(*max_day as usize);

                let mut current = NaiveDate::from_ymd_opt(current_year as i32, 1, 1).unwrap();

                while current.ordinal() <= *max_day {
                    targets.push(current);
                    current = current.succ_opt().unwrap();
                }

                targets
            }
            PredictionRange::PriorCurrentAndNextWeek => {
                let mut targets = Vec::with_capacity(21);

                let start = (now - Duration::days(7)).date_naive();
                let end = (now + Duration::days(7)).date_naive();

                let mut current = start;

                while current <= end {
                    targets.push(current);
                    current = current.succ_opt().unwrap();
                }

                targets
            }
        }
    }
}
