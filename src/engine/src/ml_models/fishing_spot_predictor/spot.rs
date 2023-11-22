use super::{spot_predict_impl, spot_train_impl};
use crate::SpotPredictorSettings;
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId, WeatherData};
use serde::Serialize;
use tracing::instrument;

pub struct FishingSpotPredictor {
    settings: SpotPredictorSettings,
}

#[derive(Debug, Serialize)]
struct PythonTrainingData {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: i32,
    pub week: u32,
}

#[derive(Debug, Serialize)]
struct PythonPredictionInput {
    pub species_group_id: SpeciesGroup,
    pub week: u32,
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
        adapter: &dyn MLModelsOutbound,
    ) -> Result<Vec<u8>, MLModelError> {
        spot_train_impl(
            self.id(),
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data, _weather, _num_cl| {
                data.into_iter()
                    .map(|v| PythonTrainingData {
                        latitude: v.latitude,
                        longitude: v.longitude,
                        species_group_id: v.species.into(),
                        week: v.week as u32,
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
        adapter: &dyn MLModelsInbound,
    ) -> Result<(), MLModelError> {
        spot_predict_impl(
            self.id(),
            &self.settings,
            model,
            adapter,
            WeatherData::Require,
            |data, _weather, _num_cl| {
                data.iter()
                    .map(|v| PythonPredictionInput {
                        species_group_id: v.species_group_id,
                        week: v.week,
                    })
                    .collect()
            },
        )
        .await
    }
}
