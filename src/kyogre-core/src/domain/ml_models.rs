use crate::{CatchLocationId, HaulId, MLModelsInbound, MLModelsOutbound};
use async_trait::async_trait;
use chrono::{Datelike, Duration, Utc};
use error_stack::{Context, Result};
use fiskeridir_rs::SpeciesGroup;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

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
    #[strum(serialize = "fishingSpotWeatherPredictor")]
    FishingSpotWeatherPredictor = 4,
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
    pub year: i32,
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
    pub model: ModelId,
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

pub enum PredictionRange {
    CurrentYear,
    CurrentWeekAndNextWeek,
    WeeksFromStartOfYear(u32),
}

pub struct PredictionTarget {
    pub week: u32,
    pub year: u32,
}

impl PredictionRange {
    pub fn prediction_targets(&self) -> Vec<PredictionTarget> {
        let now = Utc::now();
        let iso_week = now.iso_week();
        let current_week = iso_week.week();
        let current_year = iso_week.year() as u32;

        let is_end_of_year = (current_week == 52 || current_week == 53)
            && (now + Duration::weeks(1)).iso_week().year() != current_year as i32;

        match self {
            PredictionRange::CurrentYear => {
                let mut targets = Vec::with_capacity(current_week as usize);
                for i in 1..=current_week {
                    targets.push(PredictionTarget {
                        week: i,
                        year: current_year,
                    });
                }
                targets
            }
            PredictionRange::CurrentWeekAndNextWeek => {
                if is_end_of_year {
                    vec![
                        PredictionTarget {
                            week: current_week,
                            year: current_year,
                        },
                        PredictionTarget {
                            week: 1,
                            year: current_year + 1,
                        },
                    ]
                } else {
                    vec![
                        PredictionTarget {
                            week: current_week,
                            year: current_year,
                        },
                        PredictionTarget {
                            week: current_week + 1,
                            year: current_year,
                        },
                    ]
                }
            }
            PredictionRange::WeeksFromStartOfYear(max_week) => {
                let mut targets = Vec::with_capacity(*max_week as usize);
                for i in 1..=*max_week {
                    targets.push(PredictionTarget {
                        week: i,
                        year: current_year,
                    });
                }
                targets
            }
        }
    }
}
