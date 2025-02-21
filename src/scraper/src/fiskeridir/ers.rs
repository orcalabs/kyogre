use std::sync::Arc;

use async_trait::async_trait;
use fiskeridir_rs::{DataFile, FileSource};
use orca_core::Environment;

use super::FiskeridirSource;
use crate::{DataSource, Processor, Result, ScraperId, utils::prefetch_and_scrape};

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
                        Ok(processor.add_ers_por(Box::new(data)).await?)
                    }
                    DataFile::ErsDep { .. } => {
                        let data = dir.into_deserialize(&file)?;
                        Ok(processor.add_ers_dep(Box::new(data)).await?)
                    }
                    DataFile::ErsTra { .. } => {
                        let data = dir.into_deserialize(&file)?;
                        Ok(processor.add_ers_tra(Box::new(data)).await?)
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
