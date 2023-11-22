use crate::WeightPredictorSettings;
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    CatchLocationId, MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId, WeatherData,
};
use serde::Serialize;
use tracing::instrument;

use super::{weight_predict_impl, weight_train_impl};

pub struct FishingWeightPredictor {
    settings: WeightPredictorSettings,
}

#[derive(Debug, Serialize)]
struct PythonTrainingData {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: i32,
    pub week: u32,
    pub weight: f64,
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct PythonPredictionInputKey {
    pub species_group_id: i32,
    pub week: u32,
    pub year: u32,
    pub catch_location_id: CatchLocationId,
}

#[derive(Debug, Serialize)]
struct PythonPredictionInput {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: SpeciesGroup,
    pub week: u32,
}

#[async_trait]
impl MLModel for FishingWeightPredictor {
    fn id(&self) -> ModelId {
        ModelId::Weight
    }

    #[instrument(skip_all)]
    async fn train(
        &self,
        model: Vec<u8>,
        adapter: &dyn MLModelsOutbound,
    ) -> Result<Vec<u8>, MLModelError> {
        weight_train_impl(
            self.id(),
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data| {
                let data: Vec<PythonTrainingData> = data
                    .into_iter()
                    .map(|v| PythonTrainingData {
                        latitude: v.latitude,
                        longitude: v.longitude,
                        species_group_id: v.species.into(),
                        week: v.week as u32,
                        weight: v.weight,
                    })
                    .collect();

                serde_json::to_string(&data).change_context(MLModelError::DataPreparation)
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn predict(
        &self,
        model: &[u8],
        adapter: &dyn MLModelsInbound,
    ) -> Result<(), MLModelError> {
        weight_predict_impl(
            self.id(),
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data, _weather| {
                let data: Vec<PythonPredictionInput> = data
                    .iter()
                    .map(|v| PythonPredictionInput {
                        latitude: v.latitude,
                        longitude: v.longitude,
                        species_group_id: v.species_group_id,
                        week: v.week,
                    })
                    .collect();

                data
            },
        )
        .await
    }
}

impl FishingWeightPredictor {
    pub fn new(settings: WeightPredictorSettings) -> FishingWeightPredictor {
        FishingWeightPredictor { settings }
    }
}
