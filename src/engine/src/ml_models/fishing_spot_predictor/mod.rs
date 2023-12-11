use crate::{PredictionRange, TrainingOutcome};
use chrono::{Datelike, NaiveDate, Utc};
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocationWeather, FishingSpotTrainingData, HaulId, MLModelError,
    MLModelsInbound, MLModelsOutbound, ModelId, NewFishingSpotPrediction, TrainingHaul,
    TrainingOutput, WeatherData, WeatherLocationOverlap,
};
use kyogre_core::{CatchLocationId, TrainingMode};
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

#[derive(Clone)]
pub struct SpotPredictorSettings {
    pub running_in_test: bool,
    pub use_gpu: bool,
    pub training_rounds: u32,
    pub predict_batch_size: u32,
    pub range: PredictionRange,
    pub catch_locations: Vec<CatchLocationId>,
    pub single_species_mode: Option<SpeciesGroup>,
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
    settings: &SpotPredictorSettings,
    model: Vec<u8>,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: T,
) -> Result<TrainingOutput, MLModelError>
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
                        .commit_hauls_training(model_id, hauls.into_iter().collect())
                        .await
                        .change_context(MLModelError::StoreOutput)?;
                    adapter
                        .save_model(model_id, &training_output.model)
                        .await
                        .change_context(MLModelError::StoreOutput)?;
                }
            }
        },
        TrainingMode::Local => {
            training_run(
                model_id,
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
    settings: &SpotPredictorSettings,
    output: &mut TrainingOutput,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: T,
) -> Result<TrainingOutcome, MLModelError>
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
            settings.training_mode.batch_size(),
            settings.single_species_mode,
        )
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

    if training_data.is_empty() {
        return Ok(TrainingOutcome::Finished);
    }

    let overlap = match weather {
        WeatherData::Require => WeatherLocationOverlap::OnlyOverlaps,
        WeatherData::Optional => WeatherLocationOverlap::All,
    };

    let mut catch_locations = adapter
        .catch_locations(overlap)
        .await
        .change_context(MLModelError::DataPreparation)?;

    if !settings.catch_locations.is_empty() {
        catch_locations.retain(|v| settings.catch_locations.contains(&v.id));
    };

    let weather: Result<Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>, MLModelError> =
        match weather {
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
                    .await
                    .change_context(MLModelError::DataPreparation)?
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
        return Ok(TrainingOutcome::Finished);
    }

    let training_data =
        serde_json::to_string(&training_data).change_context(MLModelError::DataPreparation)?;

    let out: (Vec<u8>, Option<f64>) = Python::with_gil(|py| {
        let py_module = PyModule::from_code(py, PYTHON_FISHING_SPOT_CODE, "", "").unwrap();
        let py_main = py_module.getattr("train").unwrap();

        let model = if output.model.is_empty() {
            None
        } else {
            Some(PyByteArray::new(py, &output.model))
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
    })
    .change_context(MLModelError::Python)?;

    event!(Level::INFO, "trained on {} new hauls", hauls.len());

    output.model = out.0;
    output.best_score = out.1;

    Ok(TrainingOutcome::Progress(hauls))
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

        let species = adapter
            .species_caught_with_traal()
            .await
            .change_context(MLModelError::DataPreparation)?;

        for c in chunk {
            for s in &species {
                if let Some(single) = settings.single_species_mode {
                    if *s != single {
                        continue;
                    }
                }
                predictions.insert(PredictionInputKey {
                    species_group_id: *s,
                    date: *c,
                });
            }
        }

        let existing_predictions = adapter
            .existing_fishing_spot_predictions(model_id, current_year)
            .await
            .change_context(MLModelError::DataPreparation)?;

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

        let catch_locations = adapter
            .catch_locations(overlap)
            .await
            .change_context(MLModelError::DataPreparation)?;

        let weather: Result<Option<HashMap<NaiveDate, Vec<CatchLocationWeather>>>, MLModelError> =
            match weather {
                WeatherData::Optional => Ok(None),
                WeatherData::Require => {
                    let mut weather_map: HashMap<NaiveDate, Vec<CatchLocationWeather>> =
                        HashMap::new();

                    let cls_weather = adapter
                        .catch_locations_weather_dates(weather_keys.into_iter().collect())
                        .await
                        .change_context(MLModelError::DataPreparation)?;

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
            date: data[i].date,
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
