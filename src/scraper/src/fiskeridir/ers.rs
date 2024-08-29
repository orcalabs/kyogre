use super::FiskeridirSource;
use crate::{
    chunks::add_in_chunks, utils::prefetch_and_scrape, DataSource, Processor, Result, ScraperId,
};
use async_trait::async_trait;
use fiskeridir_rs::{DataFile, FileSource};
use orca_core::Environment;
use std::sync::Arc;

pub struct ErsScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
    environment: Environment,
}

impl ErsScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
        environment: Environment,
    ) -> Self {
        Self {
            sources,
            fiskeridir_source,
            environment,
        }
    }
}

#[async_trait]
impl DataSource for ErsScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Ers
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            Some(2020),
            |dir, file| async move {
                match file {
                    DataFile::ErsDca { .. } => {
                        let data = dir.into_deserialize(&file)?;
                        Ok(processor.add_ers_dca(Box::new(data)).await?)
                    }
                    DataFile::ErsPor { .. } => {
                        let data = dir.into_deserialize(&file)?;
                        add_in_chunks(|por| processor.add_ers_por(por), Box::new(data), 10000).await
                    }
                    DataFile::ErsDep { .. } => {
                        let data = dir.into_deserialize(&file)?;
                        add_in_chunks(|dep| processor.add_ers_dep(dep), Box::new(data), 10000).await
                    }
                    DataFile::ErsTra { .. } => {
                        let data = dir.into_deserialize(&file)?;
                        add_in_chunks(|tra| processor.add_ers_tra(tra), Box::new(data), 10000).await
                    }
                    DataFile::Landings { .. }
                    | DataFile::Vms { .. }
                    | DataFile::AquaCultureRegister => unreachable!(),
                }
            },
        )
        .await
    }
}
