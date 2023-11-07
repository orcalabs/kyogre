use crate::CatchLocationId;
use crate::{ml_models::fishing_spot_predictor::PYTHON_FISHING_SPOT_CODE, PredictionRange};

use async_trait::async_trait;
use chrono::{Datelike, Utc};
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocationWeather, HaulId, MLModel, MLModelError, MLModelsInbound,
    MLModelsOutbound, ModelId, NewFishingSpotPrediction, PredictionTarget, TrainingHaul,
    TrainingOutcome, WeatherLocationOverlap,
};
use num_traits::FromPrimitive;
use orca_core::Environment;
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::{Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use strum::EnumCount;
use tracing::{event, Level};

pub struct FishingSpotWeatherPredictor {
    training_rounds: u32,
    // We use this flag to limit the amount of prediction data we
    // produce in tests to make them finish within a reasonable timeframe
    running_in_test: bool,
    range: PredictionRange,
    use_gpu: bool,
    training_batch_size: Option<u32>,
}

#[derive(Debug)]
struct PythonTrainingData {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: i32,
    pub week: u32,
    pub weather: HashMap<CatchLocationId, CatchLocationWeather>,
    pub year: u32,
}

#[derive(Debug)]
struct PythonPredictionInput {
    pub species_group_id: i32,
    pub weather: HashMap<CatchLocationId, CatchLocationWeather>,
    pub week: u32,
    pub year: u32,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
struct CLWeatherKey {
    pub week: u32,
    pub year: u32,
}

impl FishingSpotWeatherPredictor {
    pub fn new(
        training_rounds: u32,
        environment: Environment,
        range: PredictionRange,
        training_batch_size: Option<u32>,
    ) -> FishingSpotWeatherPredictor {
        FishingSpotWeatherPredictor {
            training_rounds,
            running_in_test: matches!(environment, Environment::Test),
            range,
            use_gpu: matches!(environment, Environment::Local),
            training_batch_size,
        }
    }
}

impl PythonPredictionInput {
    fn weather_key(&self) -> CLWeatherKey {
        CLWeatherKey {
            week: self.week,
            year: self.year,
        }
    }
}

impl PythonTrainingData {
    fn weather_key(&self) -> CLWeatherKey {
        CLWeatherKey {
            week: self.week,
            year: self.year,
        }
    }
}

impl Serialize for PythonTrainingData {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;

        map.serialize_entry("longitude", &self.longitude)?;
        map.serialize_entry("latitude", &self.latitude)?;
        map.serialize_entry("species_group_id", &self.species_group_id)?;
        map.serialize_entry("week", &self.week)?;

        for (k, v) in &self.weather {
            map.serialize_entry(&format!("{}_wind_speed_10m", k.as_ref()), &v.wind_speed_10m)?;
            map.serialize_entry(
                &format!("{}_wind_direction_10m", k.as_ref()),
                &v.wind_direction_10m,
            )?;
            map.serialize_entry(
                &format!("{}_air_temperature_2m", k.as_ref()),
                &v.air_temperature_2m,
            )?;
            map.serialize_entry(
                &format!("{}_relative_humidity_2m", k.as_ref()),
                &v.relative_humidity_2m,
            )?;
            map.serialize_entry(
                &format!("{}_precipitation_amount", k.as_ref()),
                &v.precipitation_amount,
            )?;
            map.serialize_entry(
                &format!("{}_cloud_area_fraction", k.as_ref()),
                &v.cloud_area_fraction,
            )?;
        }

        map.end()
    }
}

impl Serialize for PythonPredictionInput {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(3))?;

        map.serialize_entry("species_group_id", &self.species_group_id)?;
        map.serialize_entry("week", &self.week)?;

        for (k, v) in &self.weather {
            map.serialize_entry(&format!("{}_wind_speed_10m", k.as_ref()), &v.wind_speed_10m)?;
            map.serialize_entry(
                &format!("{}_wind_direction_10m", k.as_ref()),
                &v.wind_direction_10m,
            )?;
            map.serialize_entry(
                &format!("{}_air_temperature_2m", k.as_ref()),
                &v.air_temperature_2m,
            )?;
            map.serialize_entry(
                &format!("{}_relative_humidity_2m", k.as_ref()),
                &v.relative_humidity_2m,
            )?;
            map.serialize_entry(
                &format!("{}_precipitation_amount", k.as_ref()),
                &v.precipitation_amount,
            )?;
            map.serialize_entry(
                &format!("{}_cloud_area_fraction", k.as_ref()),
                &v.cloud_area_fraction,
            )?;
        }

        map.end()
    }
}

#[async_trait]
impl MLModel for FishingSpotWeatherPredictor {
    fn id(&self) -> ModelId {
        ModelId::FishingSpotWeatherPredictor
    }

    fn prediction_targets(&self) -> Vec<PredictionTarget> {
        self.range.prediction_targets()
    }
    fn prediction_batch_size(&self) -> usize {
        10
    }

    async fn train(
        &self,
        model: &[u8],
        adapter: &dyn MLModelsOutbound,
    ) -> Result<TrainingOutcome, MLModelError> {
        let mut hauls = HashSet::new();
        let mut data: Vec<PythonTrainingData> = adapter
            .fishing_spot_predictor_training_data(self.id(), self.training_batch_size)
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
                        weather: HashMap::new(),
                        year: v.year as u32,
                    })
                } else {
                    None
                }
            })
            .collect();

        let weather_keys = data
            .iter()
            .map(|v| v.weather_key())
            .collect::<HashSet<CLWeatherKey>>();

        let catch_locations = adapter
            .catch_locations(WeatherLocationOverlap::OnlyOverlaps)
            .await
            .change_context(MLModelError::DataPreparation)?;

        let mut weather: HashMap<CLWeatherKey, Vec<CatchLocationWeather>> = HashMap::new();
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

        if data.is_empty() {
            return Ok(TrainingOutcome::Finished);
        }

        for d in &mut data {
            if let Some(w) = weather.get(&d.weather_key()) {
                // The model requires the same amount of features for all training data
                // and we only train on haul data where we have a complete weather state
                // (weather data for all catch_locations that overlaps weather locations)
                if w.len() == catch_locations.len() || self.running_in_test {
                    d.weather = w.clone().into_iter().map(|v| (v.id.clone(), v)).collect();
                }
            }
        }
        data.retain(|v| !v.weather.is_empty());

        // The current batch of training data does not contain hauls with corresponding weather data, we do not want to stop
        // the training loop as other hauls might still have weather data.
        if data.is_empty() {
            return Ok(TrainingOutcome::Progress {
                new_model: model.to_vec(),
            });
        }

        let training_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

        let new_model: Vec<u8> = Python::with_gil(|py| {
            let py_module = PyModule::from_code(py, PYTHON_FISHING_SPOT_CODE, "", "").unwrap();
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
        targets: &[PredictionTarget],
    ) -> Result<(), MLModelError> {
        if model.is_empty() {
            return Ok(());
        }

        let mut predictions = HashSet::with_capacity(SpeciesGroup::COUNT * 52);

        let species = adapter
            .species_caught_with_traal()
            .await
            .change_context(MLModelError::DataPreparation)?;

        for t in targets {
            for s in &species {
                if *s == SpeciesGroup::Ukjent {
                    continue;
                }
                predictions.insert((t.year, t.week, *s as i32));
            }
        }

        let now = Utc::now();
        let iso_week = now.iso_week();
        let current_week = iso_week.week();
        let current_year = iso_week.year() as u32;

        let existing_predictions = adapter
            .existing_fishing_spot_predictions(self.id(), current_year)
            .await
            .change_context(MLModelError::DataPreparation)?;

        for v in existing_predictions {
            if !(v.year as u32 == current_year && v.week as u32 == current_week) {
                predictions.remove(&(v.year as u32, v.week as u32, v.species));
            }
        }

        let mut data: Vec<PythonPredictionInput> = predictions
            .into_iter()
            .map(|v| PythonPredictionInput {
                year: v.0,
                week: v.1,
                species_group_id: v.2,
                weather: HashMap::new(),
            })
            .collect();

        let weather_keys = data
            .iter()
            .map(|v| v.weather_key())
            .collect::<HashSet<CLWeatherKey>>();

        let catch_locations = adapter
            .catch_locations(WeatherLocationOverlap::OnlyOverlaps)
            .await
            .change_context(MLModelError::DataPreparation)?;

        let mut weather: HashMap<CLWeatherKey, Vec<CatchLocationWeather>> = HashMap::new();
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

        for d in &mut data {
            if let Some(w) = weather.get(&d.weather_key()) {
                // The model requires the same amount of features for all training/predicton data
                // and we only train on haul data where we have a complete weather state
                // (weather data for all catch_locations that overlaps weather locations)
                if w.len() == catch_locations.len() || self.running_in_test {
                    d.weather = w.clone().into_iter().map(|v| (v.id.clone(), v)).collect();
                }
            }
        }

        data.retain(|v| !v.weather.is_empty());

        if data.is_empty() {
            event!(Level::INFO, "no new predictions to make");
            return Ok(());
        }

        let prediction_data =
            serde_json::to_string(&data).change_context(MLModelError::DataPreparation)?;

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
            species: SpeciesGroup::from_i32(data[i].species_group_id).unwrap(),
            week: data[i].week,
            year: data[i].year,
            model: self.id(),
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
