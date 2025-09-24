use super::FiskeridirSource;
use crate::{
    DataSource, Processor, Result, ScraperId, chunks::add_in_chunks, utils::prefetch_and_scrape,
};
use async_trait::async_trait;
use fiskeridir_rs::FileSource;
use orca_core::Environment;
use std::sync::Arc;

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

    async fn scrape(&self, processor: &dyn Processor) -> Result<()> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.source.clone().map(|s| vec![s]).unwrap_or_default(),
            Some(2020),
            |dir, file| async move {
                let data = dir.into_deserialize(&file)?;
                add_in_chunks(
                    |data| processor.add_aqua_culture_register(data),
                    Box::new(data),
                    10000,
                )
                .await
            },
        )
        .await
    }
}
