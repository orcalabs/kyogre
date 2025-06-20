use super::{models::Weather, timestamp_from_filename};
use crate::{DataSource, Processor, Result, ScraperId};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use csv::Reader;
use pyo3::{
    BoundObject, Python,
    ffi::c_str,
    types::{PyAnyMethods, PyDateTime, PyModule, PyTzInfo},
};
use tracing::{error, info};

pub struct WeatherScraper {}

#[async_trait]
impl DataSource for WeatherScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Weather
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        let latest = processor
            .latest_weather_timestamp()
            .await?
            // This should only happen when starting the engine locally, or during tests.
            // In those cases we don't want to scrape all weather data, so just take the last day.
            .unwrap_or_else(|| Utc::now() - Duration::days(1));

        let mut files: Vec<String> = download_weather_data(latest)?;
        files.sort();

        for file in files {
            let timestamp = timestamp_from_filename(&file)?;

            let reader = Reader::from_path(&file)?;

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
                .collect::<std::result::Result<Vec<_>, _>>()?;

            match processor.add_weather(weather).await {
                Ok(()) => info!("successfully scraped weather timestamp: {}", timestamp,),
                Err(e) => {
                    error!("failed to scrape weather timestamp: {timestamp}, error: {e}");
                    // Since we srape weather data from the latest value in the database, we don't
                    // want to continue here and potentially get holes in the dataset that would
                    // have to be patched manually.
                    return Err(e.into());
                }
            }

            if let Err(e) = tokio::fs::remove_file(file).await {
                error!("failed to delete weather file: {e:?}");
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

fn download_weather_data(latest: DateTime<Utc>) -> Result<Vec<String>> {
    let py_code = c_str!(include_str!("../../../../scripts/python/weather/main.py"));

    Ok(Python::with_gil(|py| {
        let tz = PyTzInfo::utc(py)?.into_bound();
        let py_datetime = PyDateTime::from_timestamp(py, latest.timestamp() as f64, Some(&tz))?;

        let py_module = PyModule::from_code(py, py_code, c_str!(""), c_str!(""))?;

        let py_main = py_module.getattr("main")?;

        let result = py_main.call1((py_datetime,))?;

        result.extract()
    })?)
}
