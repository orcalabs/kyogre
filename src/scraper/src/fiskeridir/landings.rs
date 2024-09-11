use super::FiskeridirSource;
use crate::{utils::prefetch_and_scrape, DataSource, Processor, Result, ScraperId};
use async_trait::async_trait;
use fiskeridir_rs::{FileSource, Landing, LandingRaw};
use orca_core::Environment;
use std::sync::Arc;

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

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            Some(2020),
            |dir, file| async move {
                let year = file.year();
                let data = dir
                    .into_deserialize::<LandingRaw>(&file)?
                    .map(move |v| v.map(|v| Landing::from_raw(v, year)));

                Ok(processor.add_landings(Box::new(data), year).await?)
            },
        )
        .await
    }
}
