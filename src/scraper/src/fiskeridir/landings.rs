use std::{collections::HashSet, sync::Arc};

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{FileSource, FiskeridirRecordIter, Landing, LandingId, LandingRaw};
use kyogre_core::{FileHash, FileHashId, HashDiff};
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct LandingScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl LandingScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
    ) -> LandingScraper {
        LandingScraper {
            sources,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for LandingScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Landings
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        for source in &self.sources {
            match self.do_scrape(processor, source).await {
                Err(e) => event!(
                    Level::ERROR,
                    "failed to scrape landings for year: {}, err: {:?}",
                    source.year(),
                    e,
                ),
                Ok(HashDiff::Changed) => event!(
                    Level::INFO,
                    "successfully scraped landings year: {}",
                    source.year()
                ),
                Ok(HashDiff::Equal) => event!(
                    Level::INFO,
                    "no changes for landings year: {}",
                    source.year()
                ),
            }
        }

        Ok(())
    }
}

impl LandingScraper {
    async fn do_scrape(
        &self,
        processor: &(dyn Processor),
        source: &FileSource,
    ) -> Result<HashDiff, ScraperError> {
        let file = self.fiskeridir_source.download(source).await?;
        let hash = file.hash().change_context(ScraperError)?;
        let hash_id = FileHashId::new(FileHash::Landings, source.year());

        let diff = self
            .fiskeridir_source
            .hash_store
            .diff(&hash_id, &hash)
            .await
            .change_context(ScraperError)?;

        match diff {
            HashDiff::Equal => Ok(HashDiff::Equal),
            HashDiff::Changed => {
                let data = file
                    .into_deserialize::<LandingRaw>()
                    .change_context(ScraperError)?;
                let landing_ids = self
                    .insert_landings_chunkwise(processor, source.year(), 1000, data)
                    .await
                    .change_context(ScraperError)?;
                processor
                    .delete_removed_landings(landing_ids, source.year())
                    .await
                    .change_context(ScraperError)?;

                self.fiskeridir_source
                    .hash_store
                    .add(&hash_id, hash)
                    .await
                    .change_context(ScraperError)?;
                Ok(HashDiff::Changed)
            }
        }
    }

    async fn insert_landings_chunkwise(
        &self,
        processor: &(dyn Processor),
        data_year: u32,
        chunk_size: usize,
        data: FiskeridirRecordIter<std::fs::File, LandingRaw>,
    ) -> Result<HashSet<LandingId>, ScraperError> {
        let mut chunk: Vec<Landing> = Vec::with_capacity(chunk_size);
        let mut landing_ids: HashSet<LandingId> = HashSet::with_capacity(chunk_size);

        for (i, item) in data.enumerate() {
            match item {
                Err(e) => {
                    event!(Level::ERROR, "failed to read data: {:?}", e);
                }
                Ok(item) => match TryInto::<Landing>::try_into(item) {
                    Err(e) => {
                        event!(Level::ERROR, "failed to convert data: {:?}", e);
                        panic!("{e}");
                    }
                    Ok(item) => {
                        landing_ids.insert(item.id.clone());
                        chunk.push(item);
                        if i % chunk_size == 0 && i > 0 {
                            processor
                                .add_landings(chunk, data_year)
                                .await
                                .change_context(ScraperError)?;
                            chunk = Vec::with_capacity(chunk_size);
                        }
                    }
                },
            }
        }

        if !chunk.is_empty() {
            processor
                .add_landings(chunk, data_year)
                .await
                .change_context(ScraperError)?;
        }

        Ok(landing_ids)
    }
}
