use crate::PredictionRange;

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, HaulId, MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId,
    NewFishingSpotPrediction, TrainingHaul, TrainingOutcome,
};
use num_traits::FromPrimitive;
use orca_core::Environment;
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::Serialize;
use std::collections::HashSet;
use strum::EnumCount;
use tracing::{event, Level};

static PYTHON_FISHING_ORACLE_CODE: &str =
    include_str!("../../../../scripts/python/fishing_predictor/fishing_spot_predictor.py");

pub struct FishingSpotPredictor {
    training_rounds: u32,
    // We use this flag to limit the amount of prediction data we
    // produce in tests to make them finish within a reasonable timeframe
    running_in_test: bool,
    range: PredictionRange,
    use_gpu: bool,
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
    pub species_group_id: i32,
    pub week: u32,
    #[serde(skip)]
    pub year: u32,
}

impl FishingSpotPredictor {
    pub fn new(
        training_rounds: u32,
        environment: Environment,
        range: PredictionRange,
    ) -> FishingSpotPredictor {
        FishingSpotPredictor {
            training_rounds,
            running_in_test: matches!(environment, Environment::Test),
            range,
            use_gpu: matches!(environment, Environment::Local),
        }
    }
}

#[async_trait]
impl MLModel for FishingSpotPredictor {
    fn id(&self) -> ModelId {
        ModelId::FishingSpotPredictor
    }

    async fn train(
        &self,
        model: &[u8],
        adapter: &dyn MLModelsOutbound,
    ) -> Result<TrainingOutcome, MLModelError> {
        let mut hauls = HashSet::new();
        let data: Vec<PythonTrainingData> = adapter
            .fishing_spot_predictor_training_data()
            .await
            .unwrap()
            .into_iter()
            .filter_map(|v| {
                hauls.insert(TrainingHaul {
                    haul_id: HaulId(v.haul_id),
                    species: v.species,
                    catch_location_id: v.catch_location_id.clone(),
                });
                if self.running_in_test || distance_to_shore(v.latitude, v.longitude) > 2000.0 {
                    Some(PythonTrainingData {
                        latitude: v.latitude,
                        longitude: v.longitude,
                        species_group_id: v.species as i32,
                        week: v.week as u32,
                    })
                } else {
                    None
                }
            })
            .collect();

        if data.is_empty() {
            return Ok(TrainingOutcome::Finished);
        }
        let training_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

        let new_model: Vec<u8> = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_ORACLE_CODE, "", "").unwrap();
            let py_main = py_module.getattr("train").unwrap();

            let model = if model.is_empty() {
                None
            } else {
                Some(PyByteArray::new(py, model))
            };

            py_main
                .call1((model, training_data, self.training_rounds, self.use_gpu))?
                .extract::<Vec<u8>>()
        })
        .change_context(MLModelError::Python)?;

        event!(Level::INFO, "trained on {} new hauls", hauls.len());

        adapter
            .commit_hauls_training(self.id(), hauls.into_iter().collect())
            .await
            .change_context(MLModelError::StoreOutput)?;

        Ok(TrainingOutcome::Progress { new_model })
    }

    async fn predict(
        &self,
        model: &[u8],
        adapter: &dyn MLModelsInbound,
    ) -> Result<(), MLModelError> {
        if model.is_empty() {
            return Ok(());
        }

        let mut predictions = HashSet::with_capacity(SpeciesGroup::COUNT * 52);

        let species = adapter
            .species_caught_with_traal()
            .await
            .change_context(MLModelError::DataPreparation)?;

        let targets = self.range.prediction_targets();

        for t in targets {
            for s in &species {
                if *s == SpeciesGroup::Ukjent {
                    continue;
                }
                predictions.insert((t.year, t.week, *s as i32));
            }
        }

        let now = Utc::now();
        let current_week = now.iso_week().week();
        let current_year = now.year() as u32;

        let existing_predictions = adapter
            .existing_fishing_spot_predictions(current_year)
            .await
            .change_context(MLModelError::DataPreparation)?;

        for v in existing_predictions {
            if !(v.year as u32 == current_year && v.week as u32 == current_week) {
                predictions.remove(&(v.year as u32, v.week as u32, v.species));
            }
        }

        let data: Vec<PythonPredictionInput> = predictions
            .into_iter()
            .map(|v| PythonPredictionInput {
                year: v.0,
                week: v.1,
                species_group_id: v.2,
            })
            .collect();

        if data.is_empty() {
            return Ok(());
        }

        let prediction_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

        let predictions = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_ORACLE_CODE, "", "")?;
            let py_main = py_module.getattr("predict")?;

            let model = PyByteArray::new(py, model);

            py_main
                .call1((model, prediction_data))?
                .extract::<Vec<Vec<f64>>>()
        })
        .change_context(MLModelError::Python)?
        .into_iter()
        .enumerate()
        .map(|(i, v)| NewFishingSpotPrediction {
            latitude: v[1],
            longitude: v[0],
            species: SpeciesGroup::from_i32(data[i].species_group_id).unwrap(),
            week: data[i].week,
            year: data[i].year,
        })
        .collect::<Vec<NewFishingSpotPrediction>>();

        event!(Level::INFO, "added {} new predictions", predictions.len());

        adapter
            .add_fishing_spot_predictions(predictions)
            .await
            .change_context(MLModelError::StoreOutput)?;

        Ok(())
    }
}
