use super::max_week;
use async_trait::async_trait;
use chrono::{Datelike, Utc};
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, HaulId, MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId,
    NewFishingSpotPrediction,
};
use num_traits::FromPrimitive;
use orca_core::Environment;
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::Serialize;
use std::collections::HashSet;
use strum::{EnumCount, IntoEnumIterator};
use tracing::{event, Level};

static PYTHON_FISHING_ORACLE_CODE: &str =
    include_str!("../../../../scripts/python/fishing_predictor/fishing_spot_predictor.py");

pub struct FishingSpotPredictor {
    training_rounds: u32,
    // We use this flag to limit the amount of prediction data we
    // produce in tests to make them finish within a reasonable timeframe
    running_in_test: bool,
    num_weeks: Option<u32>,
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
        num_weeks: Option<u32>,
    ) -> FishingSpotPredictor {
        FishingSpotPredictor {
            training_rounds,
            running_in_test: matches!(environment, Environment::Test),
            num_weeks,
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
        model: Vec<u8>,
        adapter: &dyn MLModelsOutbound,
    ) -> Result<Vec<u8>, MLModelError> {
        let mut haul_ids = HashSet::new();
        let data: Vec<PythonTrainingData> = adapter
            .fishing_spot_predictor_training_data()
            .await
            .unwrap()
            .into_iter()
            .filter_map(|v| {
                haul_ids.insert(HaulId(v.haul_id));
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
            event!(Level::INFO, "now new trainig_data, skipping training");
            return Ok(vec![]);
        }
        let training_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

        let new_model: Vec<u8> = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_ORACLE_CODE, "", "").unwrap();
            let py_main = py_module.getattr("train").unwrap();

            let model = if model.is_empty() {
                None
            } else {
                Some(PyByteArray::new(py, model.as_slice()))
            };

            py_main
                .call1((model, training_data, self.training_rounds, self.use_gpu))?
                .extract::<Vec<u8>>()
        })
        .change_context(MLModelError::Python)?;

        event!(Level::INFO, "trained on {} new hauls", haul_ids.len());

        adapter
            .commit_hauls_training(self.id(), haul_ids.into_iter().collect())
            .await
            .change_context(MLModelError::StoreOutput)?;

        Ok(new_model)
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

        let now = Utc::now();
        let current_week = now.iso_week().week();
        let current_year = now.year();
        let (max_week, is_end_of_year) = max_week(now);

        let weeks = if let Some(num_weeks) = self.num_weeks {
            1..=num_weeks
        } else {
            1..=max_week
        };

        let species = adapter
            .species_caught_with_traal()
            .await
            .change_context(MLModelError::DataPreparation)?;

        for w in weeks {
            for s in &species {
                if *s == SpeciesGroup::Ukjent {
                    continue;
                }
                predictions.insert((current_year, w, *s as i32));
            }
        }

        if is_end_of_year && self.num_weeks.is_none() {
            for s in SpeciesGroup::iter() {
                predictions.insert((current_year + 1, 1, s as i32));
            }
        }

        let existing_predictions = adapter
            .existing_fishing_spot_predictions(current_year as u32)
            .await
            .change_context(MLModelError::DataPreparation)?;

        for v in existing_predictions {
            if !(v.year == current_year && v.week as u32 == current_week) {
                predictions.remove(&(v.year, v.week as u32, v.species));
            }
        }

        let data: Vec<PythonPredictionInput> = predictions
            .into_iter()
            .map(|v| PythonPredictionInput {
                year: v.0 as u32,
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
