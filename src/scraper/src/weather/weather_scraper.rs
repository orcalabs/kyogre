use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use csv::Reader;
use error_stack::{IntoReport, Result, ResultExt};
use pyo3::{
    types::{timezone_utc, PyDateTime, PyModule},
    Python,
};
use tracing::{event, Level};

use crate::{DataSource, Processor, ScraperError, ScraperId};

use super::{models::Weather, timestamp_from_filename};

pub struct WeatherScraper {}

#[async_trait]
impl DataSource for WeatherScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Weather
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let latest = processor
            .latest_weather_timestamp()
            .await
            .change_context(ScraperError)?
            // This should only happen when starting the engine locally, or during tests.
            // In those cases we don't want to scrape all weather data, so just take the last day.
            .unwrap_or_else(|| Utc::now() - Duration::days(1));

        let mut files: Vec<String> = download_weather_data(latest).change_context(ScraperError)?;
        files.sort();

        for file in files {
            let timestamp = timestamp_from_filename(&file).change_context(ScraperError)?;

            let reader = Reader::from_path(&file)
                .into_report()
                .change_context(ScraperError)?;

            let weather = reader
                .into_deserialize::<Weather>()
                .filter_map(|w| match w {
                    Ok(w) => {
                        if w.land_area_fraction < 1.0 {
                            Some(Ok(Weather::to_core_weather(w, timestamp)))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e)),
                })
                .collect::<std::result::Result<Vec<_>, _>>()
                .into_report()
                .change_context(ScraperError)?;

            match processor
                .add_weather(weather)
                .await
                .change_context(ScraperError)
            {
                Ok(()) => event!(
                    Level::INFO,
                    "successfully scraped weather timestamp: {}",
                    timestamp,
                ),
                Err(e) => {
                    event!(
                        Level::ERROR,
                        "failed to scrape weather timestamp: {}, error: {}",
                        timestamp,
                        e,
                    );
                    // Since we srape weather data from the latest value in the database, we don't
                    // want to continue here and potentially get holes in the dataset that would
                    // have to be patched manually.
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

impl WeatherScraper {
    pub fn new() -> Self {
        Self {}
    }
}

fn download_weather_data(latest: DateTime<Utc>) -> Result<Vec<String>, PythonError> {
    let py_code = include_str!("../../../../scripts/python/weather/main.py");

    Python::with_gil(|py| {
        let py_datetime =
            PyDateTime::from_timestamp(py, latest.timestamp() as f64, Some(timezone_utc(py)))
                .into_report()
                .change_context(PythonError::DateTime(latest))?;

        let py_module = PyModule::from_code(py, py_code, "", "")
            .into_report()
            .change_context(PythonError::PyModule)?;

        let py_main = py_module
            .getattr("main")
            .into_report()
            .change_context_lazy(|| PythonError::GetAttr("main".to_string()))?;

        let result = py_main
            .call1((py_datetime,))
            .into_report()
            .change_context(PythonError::Call)?;

        result
            .extract()
            .into_report()
            .change_context(PythonError::Extract)
    })
}

#[derive(Debug)]
pub(crate) enum PythonError {
    DateTime(DateTime<Utc>),
    PyModule,
    GetAttr(String),
    Call,
    Extract,
}

impl std::error::Error for PythonError {}

impl std::fmt::Display for PythonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DateTime(d) => f.write_fmt(format_args!(
                "could not convert DateTime `{}` to PyDateTime",
                d
            )),
            Self::PyModule => f.write_str("failed to create PyModule"),
            Self::GetAttr(attr) => f.write_fmt(format_args!(
                "failed to get attribute `{}` from module",
                attr
            )),
            Self::Call => f.write_str("failed to call python function"),
            Self::Extract => f.write_str("failed to extract python type to rust type"),
        }
    }
}
