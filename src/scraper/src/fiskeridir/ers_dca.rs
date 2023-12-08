use std::sync::Arc;

use crate::{utils::prefetch_and_scrape, DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::FileSource;
use kyogre_core::FileId;
use orca_core::Environment;

use super::FiskeridirSource;

pub struct ErsDcaScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl ErsDcaScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
        environment: Environment,
    ) -> ErsDcaScraper {
        ErsDcaScraper {
            sources,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for ErsDcaScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDca
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            FileId::ErsDca,
            Some(2020),
            |_, file| async move {
                let data = file.into_deserialize().change_context(ScraperError)?;

                processor
                    .add_ers_dca(Box::new(data))
                    .await
                    .change_context(ScraperError)
            },
        )
        .await
    }
}
