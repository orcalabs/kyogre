use async_trait::async_trait;
use chrono::{Datelike, Utc};
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocation, CatchLocationId, HaulId, MLModel, MLModelError,
    MLModelsInbound, MLModelsOutbound, ModelId, NewFishingWeightPrediction, NUM_CATCH_LOCATIONS,
};
use num_traits::FromPrimitive;
use orca_core::Environment;
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use strum::{EnumCount, IntoEnumIterator};
use tracing::{event, Level};

use super::max_week;

static PYTHON_FISHING_WEIGHT_PREDICTOR_CODE: &str =
    include_str!("../../../../scripts/python/fishing_predictor/fishing_weight_predictor.py");

pub struct FishingWeightPredictor {
    training_rounds: u32,
    // We use this flag to limit the amount of prediction data we
    // produce in tests to make them finish within a reasonable timeframe
    running_in_test: bool,
    num_weeks: Option<u32>,
    num_catch_locations: Option<u32>,
    use_gpu: bool,
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
    pub species_group_id: i32,
    pub week: u32,
    #[serde(skip)]
    pub year: u32,
    #[serde(skip)]
    pub catch_location_id: CatchLocationId,
}

#[async_trait]
impl MLModel for FishingWeightPredictor {
    fn id(&self) -> ModelId {
        ModelId::FishingWeightPredictor
    }

    async fn train(
        &self,
        model: Vec<u8>,
        adapter: &dyn MLModelsOutbound,
    ) -> Result<Vec<u8>, MLModelError> {
        let mut haul_ids = HashSet::new();

        let data: Vec<PythonTrainingData> = adapter
            .fishing_weight_predictor_training_data()
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
                        weight: v.weight,
                    })
                } else {
                    None
                }
            })
            .collect();

        if data.is_empty() {
            event!(Level::INFO, "now new training_data, skipping training");
            return Ok(vec![]);
        }
        let training_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

        let new_model = Python::with_gil(|py| {
            let py_module =
                PyModule::from_code(py, PYTHON_FISHING_WEIGHT_PREDICTOR_CODE, "", "").unwrap();
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

        let now = Utc::now();
        let current_week = now.iso_week().week();
        let current_year = now.year();

        let (max_week, is_end_of_year) = max_week(now);

        let mut predictions =
            HashSet::with_capacity(SpeciesGroup::COUNT * NUM_CATCH_LOCATIONS * 52);

        let all_catch_locations: HashMap<String, CatchLocation> = adapter
            .catch_locations()
            .await
            .change_context(MLModelError::DataPreparation)?
            .into_iter()
            .map(|v| (v.id.clone().into_inner(), v))
            .collect();

        let weeks: Vec<u32> = if let Some(num_weeks) = self.num_weeks {
            (1..=num_weeks).collect()
        } else {
            (1..=max_week).collect()
        };

        let active_catch_locations = if let Some(num_catch_locations) = self.num_catch_locations {
            all_catch_locations
                .values()
                .take(num_catch_locations as usize)
                .collect::<Vec<&CatchLocation>>()
        } else {
            all_catch_locations
                .values()
                .collect::<Vec<&CatchLocation>>()
        };

        for c in &active_catch_locations {
            for w in &weeks {
                for s in SpeciesGroup::iter() {
                    predictions.insert(PythonPredictionInputKey {
                        species_group_id: s as i32,
                        week: *w,
                        year: current_year as u32,
                        catch_location_id: c.id.clone(),
                    });
                }
            }
        }

        if is_end_of_year && self.num_weeks.is_none() {
            for c in active_catch_locations {
                for s in SpeciesGroup::iter() {
                    predictions.insert(PythonPredictionInputKey {
                        species_group_id: s as i32,
                        week: 1,
                        year: (current_year + 1) as u32,
                        catch_location_id: c.id.clone(),
                    });
                }
            }
        }

        let existing_predictions = adapter
            .existing_fishing_weight_predictions(current_year as u32)
            .await
            .change_context(MLModelError::StoreOutput)?;

        for v in existing_predictions {
            if !(v.year == current_year as u32 && v.week >= current_week) {
                predictions.remove(&PythonPredictionInputKey {
                    species_group_id: v.species_group_id as i32,
                    week: v.week,
                    year: v.year,
                    catch_location_id: v.catch_location_id,
                });
            }
        }

        let data: Vec<PythonPredictionInput> = predictions
            .into_iter()
            .map(|v| {
                let cl = all_catch_locations
                    .get(v.catch_location_id.as_ref())
                    .unwrap();
                PythonPredictionInput {
                    latitude: cl.latitude,
                    longitude: cl.longitude,
                    catch_location_id: v.catch_location_id,
                    year: v.year,
                    week: v.week,
                    species_group_id: v.species_group_id,
                }
            })
            .collect();

        let prediction_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

        let predictions = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_WEIGHT_PREDICTOR_CODE, "", "")?;
            let py_main = py_module.getattr("predict")?;

            let model = PyByteArray::new(py, model);

            py_main
                .call1((model, prediction_data))?
                .extract::<Vec<f64>>()
        })
        .change_context(MLModelError::Python)?
        .into_iter()
        .enumerate()
        .map(|(i, v)| NewFishingWeightPrediction {
            catch_location_id: data[i].catch_location_id.clone(),
            species: SpeciesGroup::from_i32(data[i].species_group_id).unwrap(),
            week: data[i].week,
            weight: v,
            year: data[i].year,
        })
        .collect::<Vec<NewFishingWeightPrediction>>();

        event!(Level::INFO, "added {} new predictions", predictions.len());
        adapter
            .add_fishing_weight_predictions(predictions)
            .await
            .change_context(MLModelError::StoreOutput)?;

        Ok(())
    }
}

impl FishingWeightPredictor {
    pub fn new(
        training_rounds: u32,
        environment: Environment,
        num_weeks: Option<u32>,
        num_catch_locations: Option<u32>,
    ) -> FishingWeightPredictor {
        FishingWeightPredictor {
            training_rounds,
            running_in_test: matches!(environment, Environment::Test),
            num_weeks,
            num_catch_locations,
            use_gpu: matches!(environment, Environment::Local),
        }
    }
}
