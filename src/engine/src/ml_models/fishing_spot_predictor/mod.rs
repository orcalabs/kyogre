use crate::PredictionRange;
use chrono::{Datelike, Utc};
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocationWeather, FishingSpotTrainingData, HaulId, HaulPredictionLimit,
    MLModelError, MLModelsInbound, MLModelsOutbound, ModelId, NewFishingSpotPrediction,
    TrainingHaul, WeatherData, WeatherLocationOverlap,
};
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::{event, Level};

mod spot;
mod spot_weather;

pub use spot::*;
pub use spot_weather::*;

static PYTHON_FISHING_SPOT_CODE: &str =
    include_str!("../../../../../scripts/python/fishing_predictor/fishing_spot_predictor.py");

pub struct SpotPredictorSettings {
    pub running_in_test: bool,
    pub training_batch_size: Option<u32>,
    pub use_gpu: bool,
    pub training_rounds: u32,
    pub predict_batch_size: u32,
    pub hauls_limit_per_species: HaulPredictionLimit,
    pub range: PredictionRange,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct CLWeatherKey {
    pub week: u32,
    pub year: u32,
}

#[derive(Debug)]
struct PredictionInputKey {
    pub species_group_id: SpeciesGroup,
    pub week: u32,
    pub year: u32,
}

async fn spot_train_impl<T, S>(
    model_id: ModelId,
    settings: &SpotPredictorSettings,
    mut model: Vec<u8>,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: T,
) -> Result<Vec<u8>, MLModelError>
where
    S: Serialize,
    T: Fn(
        Vec<FishingSpotTrainingData>,
        &Option<HashMap<CLWeatherKey, Vec<CatchLocationWeather>>>,
        usize,
    ) -> Vec<S>,
{
    loop {
        let mut hauls = HashSet::new();
        let data: Vec<FishingSpotTrainingData> = adapter
            .fishing_spot_predictor_training_data(model_id, settings.training_batch_size)
            .await
            .change_context(MLModelError::DataPreparation)?
            .into_iter()
            .filter_map(|v| {
                hauls.insert(TrainingHaul {
                    haul_id: HaulId(v.haul_id),
                    species: v.species,
                    catch_location_id: v.catch_location_id.clone(),
                });
                if settings.running_in_test || distance_to_shore(v.latitude, v.longitude) > 2000.0 {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();

        if data.is_empty() {
            return Ok(model);
        }

        let catch_locations = adapter
            .catch_locations(WeatherLocationOverlap::OnlyOverlaps)
            .await
            .change_context(MLModelError::DataPreparation)?;

        let weather: Result<
            Option<HashMap<CLWeatherKey, Vec<CatchLocationWeather>>>,
            MLModelError,
        > = match weather {
            WeatherData::Optional => Ok(None),
            WeatherData::Require => {
                let mut weather: HashMap<CLWeatherKey, Vec<CatchLocationWeather>> = HashMap::new();
                let weather_keys = data
                    .iter()
                    .map(|v| CLWeatherKey {
                        week: v.week as u32,
                        year: v.year as u32,
                    })
                    .collect::<HashSet<CLWeatherKey>>();

                for v in &weather_keys {
                    for c in &catch_locations {
                        let cl_weather = adapter
                            .catch_location_weather(v.year, v.week, &c.id)
                            .await
                            .change_context(MLModelError::DataPreparation)?;

                        if let Some(w) = cl_weather {
                            weather
                                .entry(CLWeatherKey {
                                    week: v.week,
                                    year: v.year,
                                })
                                .and_modify(|v| v.push(w.clone()))
                                .or_insert(vec![w]);
                        }
                    }
                }

                Ok(Some(weather))
            }
        };

        if data.is_empty() {
            return Ok(model);
        }
        let training_data = training_data_convert(data, &(weather?), catch_locations.len());
        if training_data.is_empty() {
            continue;
        }

        let training_data =
            serde_json::to_string(&training_data).change_context(MLModelError::DataPreparation)?;

        let new_model: Vec<u8> = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_SPOT_CODE, "", "").unwrap();
            let py_main = py_module.getattr("train").unwrap();

            let model = if model.is_empty() {
                None
            } else {
                Some(PyByteArray::new(py, &model))
            };

            py_main
                .call1((
                    model,
                    training_data,
                    settings.training_rounds,
                    settings.use_gpu,
                ))?
                .extract::<Vec<u8>>()
        })
        .change_context(MLModelError::Python)?;

        event!(Level::INFO, "trained on {} new hauls", hauls.len());

        adapter
            .commit_hauls_training(model_id, hauls.into_iter().collect())
            .await
            .change_context(MLModelError::StoreOutput)?;

        model = new_model;
    }
}

async fn spot_predict_impl<T, S>(
    model_id: ModelId,
    settings: &SpotPredictorSettings,
    model: &[u8],
    adapter: &dyn MLModelsInbound,
    weather: WeatherData,
    prediction_keys_convert: T,
) -> Result<(), MLModelError>
where
    S: Serialize,
    T: Fn(
        &[PredictionInputKey],
        &Option<HashMap<CLWeatherKey, Vec<CatchLocationWeather>>>,
        usize,
    ) -> Vec<S>,
{
    if model.is_empty() {
        return Ok(());
    }

    let targets = settings.range.prediction_targets();

    let now = Utc::now();
    let iso_week = now.iso_week();
    let current_week = iso_week.week();
    let current_year = iso_week.year() as u32;

    for chunk in targets.chunks(settings.predict_batch_size as usize) {
        let mut predictions = HashSet::new();

        let species = adapter
            .species_caught_with_traal(settings.hauls_limit_per_species)
            .await
            .change_context(MLModelError::DataPreparation)?;

        for c in chunk {
            for s in &species {
                if s.weeks.contains(&c.week) {
                    predictions.insert((c.year, c.week, s.species));
                }
            }
        }

        let existing_predictions = adapter
            .existing_fishing_spot_predictions(model_id, current_year)
            .await
            .change_context(MLModelError::DataPreparation)?;

        for v in existing_predictions {
            if !(v.year as u32 == current_year && v.week as u32 == current_week) {
                predictions.remove(&(v.year as u32, v.week as u32, v.species_group_id));
            }
        }

        let data: Vec<PredictionInputKey> = predictions
            .into_iter()
            .map(|v| PredictionInputKey {
                year: v.0,
                week: v.1,
                species_group_id: v.2,
            })
            .collect();

        let weather_keys = data
            .iter()
            .map(|v| CLWeatherKey {
                week: v.week,
                year: v.year,
            })
            .collect::<HashSet<CLWeatherKey>>();

        let catch_locations = adapter
            .catch_locations(WeatherLocationOverlap::OnlyOverlaps)
            .await
            .change_context(MLModelError::DataPreparation)?;

        let weather: Result<
            Option<HashMap<CLWeatherKey, Vec<CatchLocationWeather>>>,
            MLModelError,
        > = match weather {
            WeatherData::Optional => Ok(None),
            WeatherData::Require => {
                let mut weather_map: HashMap<CLWeatherKey, Vec<CatchLocationWeather>> =
                    HashMap::new();
                for v in &weather_keys {
                    for c in &catch_locations {
                        let cl_weather = adapter
                            .catch_location_weather(v.year, v.week, &c.id)
                            .await
                            .change_context(MLModelError::DataPreparation)?;

                        if let Some(w) = cl_weather {
                            weather_map
                                .entry(CLWeatherKey {
                                    week: v.week,
                                    year: v.year,
                                })
                                .and_modify(|v| v.push(w.clone()))
                                .or_insert(vec![w]);
                        }
                    }
                }
                Ok(Some(weather_map))
            }
        };

        let prediction_data = prediction_keys_convert(&data, &(weather?), catch_locations.len());

        if prediction_data.is_empty() {
            return Ok(());
        }

        let prediction_data = serde_json::to_string(&prediction_data)
            .change_context(MLModelError::DataPreparation)?;

        let predictions = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_SPOT_CODE, "", "")?;
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
            species: data[i].species_group_id,
            week: data[i].week,
            year: data[i].year,
            model: model_id,
        })
        .collect::<Vec<NewFishingSpotPrediction>>();

        event!(Level::INFO, "added {} new predictions", predictions.len());

        adapter
            .add_fishing_spot_predictions(predictions)
            .await
            .change_context(MLModelError::StoreOutput)?;
    }
    Ok(())
}