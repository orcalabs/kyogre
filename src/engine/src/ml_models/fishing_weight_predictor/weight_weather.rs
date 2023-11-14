use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{MLModel, MLModelError, MLModelsInbound, MLModelsOutbound, ModelId, WeatherData};

use serde::Serialize;
use tracing::instrument;

use crate::WeightPredictorSettings;

use super::{weight_predict_impl, weight_train_impl, CatchLocationWeatherKey};

pub struct FishingWeightWeatherPredictor {
    settings: WeightPredictorSettings,
}

#[derive(Debug, Serialize)]
struct PythonTrainingData {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: i32,
    pub week: u32,
    pub weight: f64,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: f64,
    pub air_temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub air_pressure_at_sea_level: f64,
    pub precipitation_amount: f64,
    pub cloud_area_fraction: f64,
}

#[derive(Debug, Serialize)]
struct PythonPredictionInput {
    pub latitude: f64,
    pub longitude: f64,
    pub species_group_id: SpeciesGroup,
    pub week: u32,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: f64,
    pub air_temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub air_pressure_at_sea_level: f64,
    pub precipitation_amount: f64,
    pub cloud_area_fraction: f64,
}

#[async_trait]
impl MLModel for FishingWeightWeatherPredictor {
    fn id(&self) -> ModelId {
        ModelId::FishingWeightWeatherPredictor
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
            WeatherData::Require,
            |data| {
                let data: Vec<PythonTrainingData> = data
                    .into_iter()
                    .filter_map(|v| {
                        match (
                            v.wind_speed_10m,
                            v.wind_direction_10m,
                            v.air_temperature_2m,
                            v.relative_humidity_2m,
                            v.air_pressure_at_sea_level,
                            v.precipitation_amount,
                            v.cloud_area_fraction,
                        ) {
                            (
                                Some(wind_speed_10m),
                                Some(wind_direction_10m),
                                Some(air_temperature_2m),
                                Some(relative_humidity_2m),
                                Some(air_pressure_at_sea_level),
                                Some(precipitation_amount),
                                Some(cloud_area_fraction),
                            ) => Some(PythonTrainingData {
                                latitude: v.latitude,
                                longitude: v.longitude,
                                species_group_id: v.species.into(),
                                week: v.week as u32,
                                weight: v.weight,
                                wind_speed_10m,
                                wind_direction_10m,
                                air_temperature_2m,
                                relative_humidity_2m,
                                air_pressure_at_sea_level,
                                precipitation_amount,
                                cloud_area_fraction,
                            }),
                            _ => None,
                        }
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
            WeatherData::Require,
            |data, weather| {
                // This is safe as we require weather data for this model
                let weather = weather.as_ref().unwrap();
                let data: Vec<PythonPredictionInput> = data
                    .iter()
                    .filter_map(|value| {
                        let key = CatchLocationWeatherKey {
                            week: value.week,
                            year: value.year,
                            catch_location_id: value.catch_location_id.clone(),
                        };
                        weather.get(&key).map(|weather| PythonPredictionInput {
                            latitude: value.latitude,
                            longitude: value.longitude,
                            week: value.week,
                            species_group_id: value.species_group_id,
                            wind_speed_10m: weather.wind_speed_10m,
                            wind_direction_10m: weather.wind_speed_10m,
                            air_temperature_2m: weather.air_temperature_2m,
                            relative_humidity_2m: weather.relative_humidity_2m,
                            air_pressure_at_sea_level: weather.air_pressure_at_sea_level,
                            precipitation_amount: weather.precipitation_amount,
                            cloud_area_fraction: weather.cloud_area_fraction,
                        })
                    })
                    .collect();
                data
            },
        )
        .await
    }
}

impl FishingWeightWeatherPredictor {
    pub fn new(settings: WeightPredictorSettings) -> FishingWeightWeatherPredictor {
        FishingWeightWeatherPredictor { settings }
    }
}
