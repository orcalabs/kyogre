use chrono::{Datelike, NaiveDate, Utc};
use derivative::Derivative;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::SpeciesGroup;
use kyogre_core::{
    distance_to_shore, CatchLocation, CatchLocationId, CatchLocationWeather, HaulId, MLModelError,
    MLModelsInbound, MLModelsOutbound, ModelId, NewFishingWeightPrediction, PredictionRange,
    TrainingHaul, TrainingMode, TrainingOutput, WeatherData, WeatherLocationOverlap,
    WeightPredictorTrainingData,
};
use pyo3::{
    types::{PyByteArray, PyModule},
    Python,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tracing::{event, Level};

mod weight;
mod weight_weather;

static PYTHON_FISHING_WEIGHT_PREDICTOR_CODE: &str =
    include_str!("../../../../../scripts/python/fishing_predictor/fishing_weight_predictor.py");

pub use weight::*;
pub use weight_weather::*;

use super::CatchLocationWeatherKey;

pub enum TrainingOutcome {
    Finished,
    Progress(HashSet<TrainingHaul>),
}

#[derive(Clone)]
pub struct WeightPredictorSettings {
    pub running_in_test: bool,
    pub use_gpu: bool,
    pub training_rounds: u32,
    pub predict_batch_size: u32,
    pub range: PredictionRange,
    pub catch_locations: Vec<CatchLocationId>,
    pub single_species_mode: Option<SpeciesGroup>,
    pub training_mode: TrainingMode,
    pub test_fraction: Option<f64>,
    pub bycatch_percentage: Option<f64>,
    pub majority_species_group: bool,
}

#[derive(Debug, Derivative)]
#[derivative(Hash, Eq, PartialEq)]
struct PredictionInputKey {
    pub species_group_id: SpeciesGroup,
    pub date: NaiveDate,
    pub catch_location_id: CatchLocationId,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    pub latitude: f64,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    pub longitude: f64,
}

async fn weight_train_impl<T, S>(
    model_id: ModelId,
    settings: &WeightPredictorSettings,
    model: Vec<u8>,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: T,
) -> Result<TrainingOutput, MLModelError>
where
    S: Serialize,
    T: Fn(Vec<WeightPredictorTrainingData>) -> Vec<S>,
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
    settings: &WeightPredictorSettings,
    output: &mut TrainingOutput,
    adapter: &dyn MLModelsOutbound,
    weather: WeatherData,
    training_data_convert: &T,
) -> Result<TrainingOutcome, MLModelError>
where
    S: Serialize,
    T: Fn(Vec<WeightPredictorTrainingData>) -> Vec<S>,
{
    let mut hauls = HashSet::new();
    let training_data: Vec<WeightPredictorTrainingData> = adapter
        .fishing_weight_predictor_training_data(
            model_id,
            weather,
            settings.training_mode.batch_size(),
            settings.single_species_mode,
            settings.bycatch_percentage,
            settings.majority_species_group,
        )
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
        return Ok(TrainingOutcome::Finished);
    }

    let training_data = training_data_convert(training_data);
    if training_data.is_empty() {
        return Ok(TrainingOutcome::Progress(hauls));
    }

    let training_data =
        serde_json::to_string(&training_data).change_context(MLModelError::DataPreparation)?;

    let out: (Vec<u8>, Option<f64>) = Python::with_gil(|py| {
        let py_module =
            PyModule::from_code(py, PYTHON_FISHING_WEIGHT_PREDICTOR_CODE, "", "").unwrap();
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

async fn weight_predict_impl<T, S>(
    model_id: ModelId,
    settings: &WeightPredictorSettings,
    model: &[u8],
    adapter: &dyn MLModelsInbound,
    weather: WeatherData,
    prediction_keys_convert: T,
) -> Result<(), MLModelError>
where
    S: Serialize,
    T: Fn(
        &[PredictionInputKey],
        &Option<HashMap<CatchLocationWeatherKey, CatchLocationWeather>>,
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

        let overlap = match weather {
            WeatherData::Require => WeatherLocationOverlap::OnlyOverlaps,
            WeatherData::Optional => WeatherLocationOverlap::All,
        };

        let all_catch_locations: HashMap<String, CatchLocation> = adapter
            .catch_locations(overlap)
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
                    if let Some(single) = settings.single_species_mode {
                        if *s != single {
                            continue;
                        }
                    }
                    predictions.insert(PredictionInputKey {
                        species_group_id: *s,
                        catch_location_id: c.id.clone(),
                        latitude: c.latitude,
                        longitude: c.longitude,
                        date: *t,
                    });
                }
            }
        }

        let existing_predictions = adapter
            .existing_fishing_weight_predictions(model_id, current_year)
            .await
            .change_context(MLModelError::StoreOutput)?;

        for v in existing_predictions {
            if v.date < current_date {
                let cl = all_catch_locations
                    .get(v.catch_location_id.as_ref())
                    .unwrap();
                predictions.remove(&PredictionInputKey {
                    species_group_id: v.species_group_id,
                    date: v.date,
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
                        catch_location_id: p.catch_location_id.clone(),
                        date: p.date,
                    });
                }

                let weather_queries = weather_queries
                    .into_iter()
                    .map(|w| (w.catch_location_id, w.date))
                    .collect();

                adapter
                    .catch_locations_weather(weather_queries)
                    .await
                    .change_context(MLModelError::DataPreparation)?
                    .into_iter()
                    .for_each(|w| {
                        weather.insert(
                            CatchLocationWeatherKey {
                                catch_location_id: w.id.clone(),
                                date: w.date,
                            },
                            w,
                        );
                    });

                Ok(Some(weather))
            }
        };

        let data: Vec<PredictionInputKey> = predictions.into_iter().collect();

        let prediction_input = prediction_keys_convert(&data, &(weather?));
        if prediction_input.is_empty() {
            return Ok(());
        }

        let prediction_input = serde_json::to_string(&prediction_input)
            .change_context(MLModelError::DataPreparation)?;

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
            catch_location_id: data[i].catch_location_id.clone(),
            species: data[i].species_group_id,
            date: data[i].date,
            weight: v,
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
