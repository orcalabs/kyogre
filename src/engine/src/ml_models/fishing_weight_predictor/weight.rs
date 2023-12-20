use crate::{ml_models::lunar_value, WeightPredictorSettings};
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

use super::{weight_predict_impl, weight_train_impl};

pub struct FishingWeightPredictor {
    settings: WeightPredictorSettings,
}

#[derive(Debug, Serialize)]
struct ModelData {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: SpeciesGroup,
    pub year: u32,
    pub day: u32,
    pub num_day: u64,
    pub lunar_day_value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
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
        species: SpeciesGroup,
        adapter: &dyn MLModelsOutbound,
    ) -> Result<TrainingOutput, MLModelError> {
        weight_train_impl(
            self.id(),
            species,
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data| {
                let data: Vec<ModelData> = data
                    .into_iter()
                    .map(|v| ModelData {
                        latitude: v.latitude,
                        longitude: v.longitude,
                        species_group_id: v.species,
                        weight: Some(v.weight),
                        day: v.date.ordinal(),
                        year: v.date.year_ce().1,
                        num_day: (v.date - EARLIEST_ERS_DATE).num_days() as u64,
                        lunar_day_value: lunar_value(v.date),
                    })
                    .collect();

                data
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
        weight_predict_impl(
            self.id(),
            species,
            &self.settings,
            model,
            adapter,
            WeatherData::Optional,
            |data, _weather| {
                let data: Vec<ModelData> = data
                    .iter()
                    .map(|v| ModelData {
                        latitude: v.latitude,
                        longitude: v.longitude,
                        species_group_id: v.species_group_id,
                        day: v.date.ordinal(),
                        year: v.date.year_ce().1,
                        weight: None,
                        num_day: (v.date - EARLIEST_ERS_DATE).num_days() as u64,
                        lunar_day_value: lunar_value(v.date),
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
