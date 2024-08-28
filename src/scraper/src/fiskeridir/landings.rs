use std::sync::Arc;

use crate::{utils::prefetch_and_scrape, DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{FileSource, Landing, LandingRaw};
use orca_core::Environment;

use super::FiskeridirSource;

pub struct LandingScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl LandingScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
        environment: Environment,
    ) -> LandingScraper {
        LandingScraper {
            sources,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for LandingScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Landings
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            Some(2020),
            |dir, file| async move {
                let year = file.year();
                let data = dir
                    .into_deserialize::<LandingRaw>(&file)
                    .change_context(ScraperError)?
                    .map(move |v| match v {
                        Ok(v) => Landing::try_from_raw(v, year),
                        Err(e) => Err(e),
                    });

                processor
                    .add_landings(Box::new(data), year)
                    .await
                    .change_context(ScraperError)
            },
        )
        .await
    }
}
