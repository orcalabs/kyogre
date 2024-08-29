use super::FiskeridirSource;
use crate::{
    chunks::add_in_chunks, utils::prefetch_and_scrape, DataSource, Processor, Result, ScraperId,
};
use async_trait::async_trait;
use fiskeridir_rs::FileSource;
use orca_core::Environment;
use std::sync::Arc;

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
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            Some(2023),
            |dir, file| async move {
                let data = dir.into_deserialize(&file)?;
                add_in_chunks(|vms| processor.add_vms(vms), Box::new(data), 10000).await
            },
        )
        .await
    }
}
