use chrono::{Datelike, Utc};
use derivative::Derivative;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocation, CatchLocationId, CatchLocationWeather, HaulId, MLModelError,
    MLModelsInbound, MLModelsOutbound, ModelId, NewFishingWeightPrediction, PredictionRange,
    TrainingHaul, WeatherData, WeatherLocationOverlap, WeightPredictorTrainingData,
};
use num_traits::FromPrimitive;
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use std::collections::{HashMap, HashSet};
use tracing::{event, Level};

mod weight;
mod weight_weather;

static PYTHON_FISHING_WEIGHT_PREDICTOR_CODE: &str =
    include_str!("../../../../../scripts/python/fishing_predictor/fishing_weight_predictor.py");

pub use weight::*;
pub use weight_weather::*;

use super::CatchLocationWeatherKey;

pub struct WeightPredictorSettings {
    pub running_in_test: bool,
    pub training_batch_size: Option<u32>,
    pub use_gpu: bool,
    pub training_rounds: u32,
    pub predict_batch_size: u32,
    pub range: PredictionRange,
    pub catch_locations: Vec<CatchLocationId>,
}

#[derive(Debug, Derivative)]
#[derivative(Hash, Eq, PartialEq)]
struct PredictionInputKey {
    pub species_group_id: i32,
    pub week: u32,
    pub year: u32,
    pub catch_location_id: CatchLocationId,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    pub latitude: f64,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    pub longitude: f64,
}

async fn weight_train_impl<T>(
    model_id: ModelId,
    settings: &WeightPredictorSettings,
    mut model: Vec<u8>,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_to_json: T,
) -> Result<Vec<u8>, MLModelError>
where
    T: Fn(Vec<WeightPredictorTrainingData>) -> Result<String, MLModelError>,
{
    loop {
        let mut hauls = HashSet::new();
        let training_data: Vec<WeightPredictorTrainingData> = adapter
            .fishing_weight_predictor_training_data(model_id, weather, settings.training_batch_size)
            .await
            .change_context(MLModelError::DataPreparation)?
            .into_iter()
            .filter_map(|v| {
                hauls.insert(TrainingHaul {
                    haul_id: HaulId(v.haul_id),
                    species: v.species,
                    catch_location_id: v.catch_location.clone(),
                });
                if settings.running_in_test || distance_to_shore(v.latitude, v.longitude) > 2000.0 {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();

        if training_data.is_empty() {
            return Ok(model);
        }
        let training_data = training_data_to_json(training_data)?;
        if training_data.is_empty() {
            continue;
        }

        let new_model = Python::with_gil(|py| {
            let py_module =
                PyModule::from_code(py, PYTHON_FISHING_WEIGHT_PREDICTOR_CODE, "", "").unwrap();
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

        adapter
            .save_model(model_id, &new_model)
            .await
            .change_context(MLModelError::StoreOutput)?;

        model = new_model;
    }
}

async fn weight_predict_impl<T>(
    model_id: ModelId,
    settings: &WeightPredictorSettings,
    model: &[u8],
    adapter: &dyn MLModelsInbound,
    weather: WeatherData,
    prediction_keys_to_json: T,
) -> Result<(), MLModelError>
where
    T: Fn(
        &[PredictionInputKey],
        &Option<HashMap<CatchLocationWeatherKey, CatchLocationWeather>>,
    ) -> Result<String, MLModelError>,
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

        let all_catch_locations: HashMap<String, CatchLocation> = adapter
            .catch_locations(WeatherLocationOverlap::All)
            .await
            .change_context(MLModelError::DataPreparation)?
            .into_iter()
            .map(|v| (v.id.clone().into_inner(), v))
            .collect();

        let active_catch_locations = if settings.catch_locations.is_empty() {
            all_catch_locations
                .values()
                .collect::<Vec<&CatchLocation>>()
        } else {
            all_catch_locations
                .values()
                .filter(|v| settings.catch_locations.contains(&v.id))
                .collect::<Vec<&CatchLocation>>()
        };

        let species = adapter
            .species_caught_with_traal()
            .await
            .change_context(MLModelError::DataPreparation)?;

        for t in chunk {
            for c in &active_catch_locations {
                for s in &species {
                    predictions.insert(PredictionInputKey {
                        species_group_id: *s as i32,
                        week: t.week,
                        year: t.year,
                        catch_location_id: c.id.clone(),
                        latitude: c.latitude,
                        longitude: c.longitude,
                    });
                }
            }
        }

        let existing_predictions = adapter
            .existing_fishing_weight_predictions(model_id, current_year)
            .await
            .change_context(MLModelError::StoreOutput)?;

        for v in existing_predictions {
            if !(v.year == current_year && v.week >= current_week) {
                let cl = all_catch_locations
                    .get(v.catch_location_id.as_ref())
                    .unwrap();
                predictions.remove(&PredictionInputKey {
                    species_group_id: v.species_group_id as i32,
                    week: v.week,
                    year: v.year,
                    catch_location_id: v.catch_location_id,
                    latitude: cl.latitude,
                    longitude: cl.longitude,
                });
            }
        }

        let weather: Result<
            Option<HashMap<CatchLocationWeatherKey, CatchLocationWeather>>,
            MLModelError,
        > = match weather {
            WeatherData::Optional => Ok(None),
            WeatherData::Require => {
                let mut weather: HashMap<CatchLocationWeatherKey, CatchLocationWeather> =
                    HashMap::new();
                let mut weather_queries: HashSet<CatchLocationWeatherKey> = HashSet::new();
                for p in &predictions {
                    weather_queries.insert(CatchLocationWeatherKey {
                        week: p.week,
                        year: p.year,
                        catch_location_id: p.catch_location_id.clone(),
                    });
                }
                for v in weather_queries {
                    let cl_weather = adapter
                        .catch_location_weather(v.year, v.week, &v.catch_location_id)
                        .await
                        .change_context(MLModelError::DataPreparation)?;
                    if let Some(cl_weather) = cl_weather {
                        weather.insert(v, cl_weather);
                    }
                }
                Ok(Some(weather))
            }
        };

        let prediction_data: Vec<PredictionInputKey> = predictions.into_iter().collect();
        let prediction_input = prediction_keys_to_json(&prediction_data, &(weather?))?;

        let predictions = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_WEIGHT_PREDICTOR_CODE, "", "")?;
            let py_main = py_module.getattr("predict")?;

            let model = PyByteArray::new(py, model);

            py_main
                .call1((model, prediction_input))?
                .extract::<Vec<f64>>()
        })
        .change_context(MLModelError::Python)?
        .into_iter()
        .enumerate()
        .map(|(i, v)| NewFishingWeightPrediction {
            catch_location_id: prediction_data[i].catch_location_id.clone(),
            species: SpeciesGroup::from_i32(prediction_data[i].species_group_id).unwrap(),
            week: prediction_data[i].week,
            weight: v,
            year: prediction_data[i].year,
            model: model_id,
        })
        .collect::<Vec<NewFishingWeightPrediction>>();

        event!(Level::INFO, "added {} new predictions", predictions.len());
        adapter
            .add_fishing_weight_predictions(predictions)
            .await
            .change_context(MLModelError::StoreOutput)?;
    }

    Ok(())
}
