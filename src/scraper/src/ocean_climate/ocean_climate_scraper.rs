use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use csv::Reader;
use pyo3::{
    BoundObject, Python,
    ffi::c_str,
    types::{PyAnyMethods, PyDateTime, PyModule, PyTzInfo},
};
use tracing::{error, info};

use crate::{DataSource, Processor, Result, ScraperId};

use super::{models::OceanClimate, timestamp_and_depth_from_filename};

pub struct OceanClimateScraper {}

#[async_trait]
impl DataSource for OceanClimateScraper {
    fn id(&self) -> ScraperId {
        ScraperId::OceanClimate
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        let latest = processor
            .latest_ocean_climate_timestamp()
            .await?
            // This should only happen when starting the engine locally, or during tests.
            // In those cases we don't want to scrape all ocean_climate data, so just take the last day.
            .unwrap_or_else(|| Utc::now() - Duration::days(1));

        let mut files: Vec<String> = download_ocean_climate_data(latest)?;
        files.sort();

        for file in files {
            let (timestamp, depth) = timestamp_and_depth_from_filename(&file)?;

            let reader = Reader::from_path(&file)?;

            let ocean_climate = reader
                .into_deserialize::<OceanClimate>()
                .map(|o| match o {
                    Ok(o) => Ok(OceanClimate::to_core_ocean_climate(o, timestamp, depth)),
                    Err(e) => Err(e),
                })
                .collect::<std::result::Result<Vec<_>, _>>()?;

            match processor.add_ocean_climate(ocean_climate).await {
                Ok(()) => info!(
                    "successfully scraped ocean_climate timestamp / depth: {} / {}",
                    timestamp, depth,
                ),
                Err(e) => {
                    error!(
                        "failed to scrape ocean_climate timestamp / depth: {timestamp} / {depth}, error: {e:?}"
                    );
                    // Since we srape ocean_climate data from the latest value in the database, we don't
                    // want to continue here and potentially get holes in the dataset that would
                    // have to be patched manually.
                    return Err(e.into());
                }
            }

            if let Err(e) = tokio::fs::remove_file(file).await {
                error!("failed to delete ocean_climate file: {e:?}");
            }
        }

        Ok(())
    }
}

impl OceanClimateScraper {
    pub fn new() -> Self {
        Self {}
    }
}

fn download_ocean_climate_data(latest: DateTime<Utc>) -> Result<Vec<String>> {
    let py_code = c_str!(include_str!(
        "../../../../scripts/python/ocean_climate/main.py"
    ));

    Ok(Python::with_gil(|py| {
        let tz = PyTzInfo::utc(py)?.into_bound();
        let py_datetime = PyDateTime::from_timestamp(py, latest.timestamp() as f64, Some(&tz))?;

        let py_module = PyModule::from_code(py, py_code, c_str!(""), c_str!(""))?;

        let py_main = py_module.getattr("main")?;

        let result = py_main.call1((py_datetime,))?;

        result.extract()
    })?)
}
