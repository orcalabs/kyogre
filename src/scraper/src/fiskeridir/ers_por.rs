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

pub struct ErsPorScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl ErsPorScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
        environment: Environment,
    ) -> ErsPorScraper {
        ErsPorScraper {
            sources,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for ErsPorScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsPor
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            FileId::ErsPor,
            Some(2020),
            |_, file| async move {
                let data = file.into_deserialize().change_context(ScraperError)?;
                add_in_chunks(
                    |ers_posr| processor.add_ers_por(ers_posr),
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
