use std::sync::Arc;

use crate::{
    chunks::add_in_chunks, utils::prefetch_and_scrape, DataSource, Processor, ScraperError,
    ScraperId,
};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::FileSource;
use kyogre_core::FileId;
use orca_core::Environment;

use super::FiskeridirSource;

pub struct ErsTraScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl ErsTraScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
        environment: Environment,
    ) -> ErsTraScraper {
        ErsTraScraper {
            sources,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for ErsTraScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsTra
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            FileId::ErsTra,
            Some(2020),
            |_, file| async move {
                let data = file.into_deserialize().change_context(ScraperError)?;
                add_in_chunks(
                    |ers_tra| processor.add_ers_tra(ers_tra),
                    Box::new(data),
                    10000,
                )
                .await
                .change_context(ScraperError)
            },
        )
        .await
    }
}
