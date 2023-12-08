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

pub struct AquaCultureRegisterScraper {
    source: Option<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl AquaCultureRegisterScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        source: Option<FileSource>,
        environment: Environment,
    ) -> AquaCultureRegisterScraper {
        AquaCultureRegisterScraper {
            source,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for AquaCultureRegisterScraper {
    fn id(&self) -> ScraperId {
        ScraperId::AquaCultureRegister
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.source.clone().map(|s| vec![s]).unwrap_or_default(),
            FileId::AquaCultureRegister,
            Some(2020),
            |_, file| async move {
                let data = file.into_deserialize().change_context(ScraperError)?;
                add_in_chunks(
                    |data| processor.add_aqua_culture_register(data),
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
