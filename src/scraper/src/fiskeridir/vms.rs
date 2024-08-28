use std::sync::Arc;

use crate::{
    chunks::add_in_chunks, utils::prefetch_and_scrape, DataSource, Processor, ScraperError,
    ScraperId,
};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::FileSource;
use orca_core::Environment;

use super::FiskeridirSource;

pub struct VmsScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl VmsScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
        environment: Environment,
    ) -> VmsScraper {
        VmsScraper {
            sources,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for VmsScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Vms
    }
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            Some(2023),
            |dir, file| async move {
                let data = dir.into_deserialize(&file).change_context(ScraperError)?;
                add_in_chunks(|vms| processor.add_vms(vms), Box::new(data), 10000)
                    .await
                    .change_context(ScraperError)
            },
        )
        .await
    }
}
