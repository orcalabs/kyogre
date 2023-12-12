use crate::{CatchLocationId, SpotPredictorSettings};

use async_trait::async_trait;
use chrono::Datelike;
use error_stack::Result;
use fiskeridir_rs::SpeciesGroup;
use itertools::Itertools;
use kyogre_core::{
    CatchLocationWeather, MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId,
    TrainingOutput, WeatherData,
};
use serde::{Serialize, Serializer};
use std::collections::HashMap;

use super::{spot_predict_impl, spot_train_impl};

pub struct FishingSpotWeatherPredictor {
    settings: SpotPredictorSettings,
}

#[derive(Debug)]
struct PythonTrainingData {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: i32,
    pub day: u32,
    pub year: u32,
    pub weather: HashMap<CatchLocationId, CatchLocationWeather>,
}

#[derive(Debug)]
struct PythonPredictionInput {
    pub species_group_id: SpeciesGroup,
    pub weather: HashMap<CatchLocationId, CatchLocationWeather>,
    pub year: u32,
    pub day: u32,
}

impl FishingSpotWeatherPredictor {
    pub fn new(settings: SpotPredictorSettings) -> FishingSpotWeatherPredictor {
        FishingSpotWeatherPredictor { settings }
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
        map.serialize_entry("day", &self.day)?;
        map.serialize_entry("year", &self.year)?;

        for (k, v) in self.weather.iter().sorted_by_key(|(k, _v)| (*k).clone()) {
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
        map.serialize_entry("day", &self.day)?;
        map.serialize_entry("year", &self.year)?;

        for (k, v) in self.weather.iter().sorted_by_key(|(k, _v)| (*k).clone()) {
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
        ModelId::SpotWeather
    }

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
            WeatherData::Require,
            |data, weather, num_cl| {
                // This is safe as we require weather data for this model
                let weather = weather.as_ref().unwrap();
                data.iter()
                    .filter_map(|v| {
                        if let Some(w) = weather.get(&v.date) {
                            // The model requires the same amount of features for all training/predicton data
                            // and we only train on haul data where we have a complete weather state
                            // (weather data for all catch_locations that overlaps weather locations)
                            if w.len() == num_cl || self.settings.running_in_test {
                                Some(PythonTrainingData {
                                    species_group_id: v.species.into(),
                                    weather: w
                                        .clone()
                                        .into_iter()
                                        .map(|v| (v.id.clone(), v))
                                        .collect(),
                                    day: v.date.ordinal(),
                                    year: v.date.year_ce().1,
                                    longitude: v.longitude,
                                    latitude: v.latitude,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            },
        )
        .await
    }

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
            WeatherData::Require,
            |data, weather, num_cl| {
                // This is safe as we require weather data for this model
                let weather = weather.as_ref().unwrap();
                data.iter()
                    .filter_map(|v| {
                        if let Some(w) = weather.get(&v.date) {
                            // The model requires the same amount of features for all training/predicton data
                            // and we only train on haul data where we have a complete weather state
                            // (weather data for all catch_locations that overlaps weather locations)
                            if w.len() == num_cl || self.settings.running_in_test {
                                Some(PythonPredictionInput {
                                    species_group_id: v.species_group_id,
                                    weather: w
                                        .clone()
                                        .into_iter()
                                        .map(|v| (v.id.clone(), v))
                                        .collect(),
                                    day: v.date.ordinal(),
                                    year: v.date.year_ce().1,
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect()
            },
        )
        .await
    }
}
