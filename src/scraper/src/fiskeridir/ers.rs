use std::sync::Arc;

use crate::{
    chunks::add_in_chunks, utils::prefetch_and_scrape, DataSource, Processor, ScraperError,
    ScraperId,
};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{DataFile, FileSource};
use orca_core::Environment;

use super::FiskeridirSource;

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

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        prefetch_and_scrape(
            self.environment,
            self.fiskeridir_source.clone(),
            self.sources.clone(),
            Some(2020),
            |dir, file| async move {
                match file {
                    DataFile::ErsDca { year } => {
                        if year == 2024 {
                            return Ok(());
                        }
                        let data = dir.into_deserialize(&file).change_context(ScraperError)?;
                        processor
                            .add_ers_dca(Box::new(data))
                            .await
                            .change_context(ScraperError)
                    }
                    DataFile::ErsPor { .. } => {
                        let data = dir.into_deserialize(&file).change_context(ScraperError)?;
                        add_in_chunks(|por| processor.add_ers_por(por), Box::new(data), 10000)
                            .await
                            .change_context(ScraperError)
                    }
                    DataFile::ErsDep { .. } => {
                        let data = dir.into_deserialize(&file).change_context(ScraperError)?;
                        add_in_chunks(|dep| processor.add_ers_dep(dep), Box::new(data), 10000)
                            .await
                            .change_context(ScraperError)
                    }
                    DataFile::ErsTra { .. } => {
                        let data = dir.into_deserialize(&file).change_context(ScraperError)?;
                        add_in_chunks(|tra| processor.add_ers_tra(tra), Box::new(data), 10000)
                            .await
                            .change_context(ScraperError)
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
