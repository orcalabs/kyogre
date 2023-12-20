use super::{spot_predict_impl, spot_train_impl};
use crate::{ml_models::lunar_value, SpotPredictorSettings};
use async_trait::async_trait;
use chrono::Datelike;
use error_stack::Result;
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId, TrainingOutput, WeatherData,
    EARLIEST_ERS_DATE,
};
use serde::Serialize;
use tracing::instrument;

pub struct FishingSpotPredictor {
    settings: SpotPredictorSettings,
}

#[derive(Debug, Serialize)]
struct PythonTrainingData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    pub species_group_id: SpeciesGroup,
    pub day: u32,
    pub year: u32,
    pub num_day: u64,
    pub lunar_day_value: f64,
}

impl FishingSpotPredictor {
    pub fn new(settings: SpotPredictorSettings) -> FishingSpotPredictor {
        FishingSpotPredictor { settings }
    }
}

#[async_trait]
impl MLModel for FishingSpotPredictor {
    fn id(&self) -> ModelId {
        ModelId::Spot
    }

    #[instrument(skip_all)]
    async fn train(
        &self,
        model: Vec<u8>,
        species: SpeciesGroup,
        adapter: &dyn MLModelsOutbound,
    ) -> Result<TrainingOutput, MLModelError> {
        spot_train_impl(
            self.id(),
            species,
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data, _weather, _num_cl| {
                data.into_iter()
                    .map(|v| PythonTrainingData {
                        latitude: Some(v.latitude),
                        longitude: Some(v.longitude),
                        species_group_id: v.species,
                        day: v.date.ordinal(),
                        year: v.date.year_ce().1,
                        num_day: (v.date - EARLIEST_ERS_DATE).num_days() as u64,
                        lunar_day_value: lunar_value(v.date),
                    })
                    .collect::<Vec<PythonTrainingData>>()
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn predict(
        &self,
        model: &[u8],
        species: SpeciesGroup,
        adapter: &dyn MLModelsInbound,
    ) -> Result<(), MLModelError> {
        spot_predict_impl(
            self.id(),
            species,
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data, _weather, _num_cl| {
                data.iter()
                    .map(|v| PythonTrainingData {
                        species_group_id: v.species_group_id,
                        latitude: None,
                        longitude: None,
                        day: v.date.ordinal(),
                        year: v.date.year_ce().1,
                        num_day: (v.date - EARLIEST_ERS_DATE).num_days() as u64,
                        lunar_day_value: lunar_value(v.date),
                    })
                    .collect()
            },
        )
        .await
    }
}
