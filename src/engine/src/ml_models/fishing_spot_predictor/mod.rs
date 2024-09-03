use crate::{error::Result, PredictionRange, TrainingOutcome};
use chrono::{Datelike, NaiveDate, Utc};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocationWeather, FishingSpotTrainingData, MLModelsInbound,
    MLModelsOutbound, ModelId, NewFishingSpotPrediction, TrainingHaul, TrainingOutput, WeatherData,
    WeatherLocationOverlap,
};
use kyogre_core::{CatchLocationId, TrainingMode};
use pyo3::types::PyAnyMethods;
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::info;

mod spot;
mod spot_weather;

pub use spot::*;
pub use spot_weather::*;

static PYTHON_FISHING_SPOT_CODE: &str =
    include_str!("../../../../../scripts/python/fishing_predictor/fishing_spot_predictor.py");

#[derive(Clone)]
pub struct SpotPredictorSettings {
    pub running_in_test: bool,
    pub use_gpu: bool,
    pub training_rounds: u32,
    pub predict_batch_size: u32,
    pub range: PredictionRange,
    pub catch_locations: Vec<CatchLocationId>,
    pub training_mode: TrainingMode,
    pub test_fraction: Option<f64>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct PredictionInputKey {
    pub species_group_id: SpeciesGroup,
    pub date: NaiveDate,
}

async fn spot_train_impl<T, S>(
    model_id: ModelId,
    species: SpeciesGroup,
    settings: &SpotPredictorSettings,
    model: Vec<u8>,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: T,
) -> Result<TrainingOutput>
where
    S: Serialize,
    T: Fn(
        Vec<FishingSpotTrainingData>,
        &Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>,
        usize,
    ) -> Vec<S>,
{
    let mut training_output = TrainingOutput {
        model,
        best_score: None,
    };

    match settings.training_mode {
        TrainingMode::Single | TrainingMode::Batches(_) => loop {
            match training_run(
                model_id,
                species,
                settings,
                &mut training_output,
                adapter,
                weather,
                &training_data_convert,
            )
            .await?
            {
                TrainingOutcome::Finished => break,
                TrainingOutcome::Progress(hauls) => {
                    adapter
                        .commit_hauls_training(model_id, species, hauls.into_iter().collect())
                        .await?;
                    adapter
                        .save_model(model_id, &training_output.model, species)
                        .await?;
                }
            }
        },
        TrainingMode::Local => {
            training_run(
                model_id,
                species,
                settings,
                &mut training_output,
                adapter,
                weather,
                &training_data_convert,
            )
            .await?;
        }
    }

    Ok(training_output)
}

async fn training_run<T, S>(
    model_id: ModelId,
    species: SpeciesGroup,
    settings: &SpotPredictorSettings,
    output: &mut TrainingOutput,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: T,
) -> Result<TrainingOutcome>
where
    S: Serialize,
    T: Fn(
        Vec<FishingSpotTrainingData>,
        &Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>,
        usize,
    ) -> Vec<S>,
{
    let mut hauls = HashSet::new();
    let training_data: Vec<FishingSpotTrainingData> = adapter
        .fishing_spot_predictor_training_data(
            model_id,
            species,
            settings.training_mode.batch_size(),
        )
        .await?
        .into_iter()
        .filter_map(|v| {
            hauls.insert(TrainingHaul {
                haul_id: v.haul_id,
                catch_location_id: v.catch_location_id.clone(),
            });
            if settings.running_in_test || distance_to_shore(v.latitude, v.longitude) > 2000.0 {
                Some(v)
            } else {
                None
            }
        })
        .collect();

    if training_data.is_empty() {
        return Ok(TrainingOutcome::Finished);
    }

    let overlap = match weather {
        WeatherData::Require => WeatherLocationOverlap::OnlyOverlaps,
        WeatherData::Optional => WeatherLocationOverlap::All,
    };

    let mut catch_locations = adapter.catch_locations(overlap).await?;

    if !settings.catch_locations.is_empty() {
        catch_locations.retain(|v| settings.catch_locations.contains(&v.id));
    };

    let weather: Result<Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>> = match weather {
        WeatherData::Optional => Ok(None),
        WeatherData::Require => {
            let mut weather: HashMap<NaiveDate, Vec<CatchLocationWeather>> = HashMap::new();
            let weather_dates = training_data
                .iter()
                .map(|v| v.date)
                .collect::<HashSet<NaiveDate>>()
                .into_iter()
                .collect::<Vec<NaiveDate>>();

            adapter
                .catch_locations_weather_dates(weather_dates)
                .await?
                .into_iter()
                .for_each(|w| {
                    weather
                        .entry(w.date)
                        .and_modify(|v| v.push(w.clone()))
                        .or_insert(vec![w]);
                });

            Ok(Some(weather))
        }
    };

    let training_data = training_data_convert(training_data, &(weather?), catch_locations.len());
    if training_data.is_empty() {
        return Ok(TrainingOutcome::Progress(hauls));
    }

    let training_data = serde_json::to_string(&training_data)?;

    let out: (Vec<u8>, Option<f64>) = Python::with_gil(|py| {
        let py_module = PyModule::from_code_bound(py, PYTHON_FISHING_SPOT_CODE, "", "").unwrap();
        let py_main = py_module.getattr("train").unwrap();

        let model = if output.model.is_empty() {
            None
        } else {
            Some(PyByteArray::new_bound(py, &output.model))
        };

        py_main
            .call1((
                model,
                training_data,
                settings.training_rounds,
                settings.use_gpu,
                settings.test_fraction,
            ))?
            .extract::<(Vec<u8>, Option<f64>)>()
    })?;

    info!("trained on {} new hauls", hauls.len());

    output.model = out.0;
    output.best_score = out.1;

    Ok(TrainingOutcome::Progress(hauls))
}

async fn spot_predict_impl<T, S>(
    model_id: ModelId,
    species: SpeciesGroup,
    settings: &SpotPredictorSettings,
    model: &[u8],
    adapter: &dyn MLModelsInbound,
    weather: WeatherData,
    prediction_keys_convert: T,
) -> Result<()>
where
    S: Serialize,
    T: Fn(
        &[PredictionInputKey],
        &Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>,
        usize,
    ) -> Vec<S>,
{
    if model.is_empty() {
        return Ok(());
    }

    let targets = settings.range.prediction_dates();

    let now = Utc::now();
    let current_date = now.date_naive();
    let current_year = now.year() as u32;

    for chunk in targets.chunks(settings.predict_batch_size as usize) {
        let mut predictions = HashSet::new();

        for c in chunk {
            predictions.insert(PredictionInputKey {
                species_group_id: species,
                date: *c,
            });
        }

        let existing_predictions = adapter
            .existing_fishing_spot_predictions(model_id, species, current_year)
            .await?;

        for v in existing_predictions {
            if v.date < current_date {
                predictions.remove(&PredictionInputKey {
                    species_group_id: v.species_group_id,
                    date: v.date,
                });
            }
        }

        let data: Vec<PredictionInputKey> = predictions.into_iter().collect();

        let weather_keys = data.iter().map(|v| v.date).collect::<HashSet<NaiveDate>>();

        let overlap = match weather {
            WeatherData::Require => WeatherLocationOverlap::OnlyOverlaps,
            WeatherData::Optional => WeatherLocationOverlap::All,
        };

        let catch_locations = adapter.catch_locations(overlap).await?;

        let weather: Result<Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>> = match weather {
            WeatherData::Optional => Ok(None),
            WeatherData::Require => {
                let mut weather_map: HashMap<NaiveDate, Vec<CatchLocationWeather>> = HashMap::new();

                let cls_weather = adapter
                    .catch_locations_weather_dates(weather_keys.into_iter().collect())
                    .await?;

                for w in cls_weather {
                    weather_map
                        .entry(w.date)
                        .and_modify(|v| v.push(w.clone()))
                        .or_insert(vec![w]);
                }

                Ok(Some(weather_map))
            }
        };

        let prediction_data = prediction_keys_convert(&data, &(weather?), catch_locations.len());

        if prediction_data.is_empty() {
            return Ok(());
        }

        let prediction_data = serde_json::to_string(&prediction_data)?;

        let predictions = Python::with_gil(|py| {
            let py_module = PyModule::from_code_bound(py, PYTHON_FISHING_SPOT_CODE, "", "")?;
            let py_main = py_module.getattr("predict")?;

            let model = PyByteArray::new_bound(py, model);

            py_main
                .call1((model, prediction_data))?
                .extract::<Vec<Vec<f64>>>()
        })?
        .into_iter()
        .enumerate()
        .map(|(i, v)| NewFishingSpotPrediction {
            latitude: v[1],
            longitude: v[0],
            species: data[i].species_group_id,
            date: data[i].date,
            model: model_id,
        })
        .collect::<Vec<NewFishingSpotPrediction>>();

        info!("added {} new predictions", predictions.len());

        adapter.add_fishing_spot_predictions(predictions).await?;
    }

    Ok(())
}
