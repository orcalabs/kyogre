use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::FileSource;
use kyogre_core::{FileHash, FileHashId, HashDiff};
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct ErsDcaScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl ErsDcaScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
    ) -> ErsDcaScraper {
        ErsDcaScraper {
            sources,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for ErsDcaScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDca
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        for source in &self.sources {
            let file = self.fiskeridir_source.download(source).await?;
            let hash = file.hash().change_context(ScraperError)?;
            let hash_id = FileHashId::new(FileHash::ErsDca, source.year());

            let diff = self
                .fiskeridir_source
                .hash_store
                .diff(&hash_id, &hash)
                .await
                .change_context(ScraperError)?;

            match diff {
                HashDiff::Equal => event!(
                    Level::INFO,
                    "no changes for ers_dca year: {}",
                    source.year()
                ),
                HashDiff::Changed => {
                    let data = file.into_deserialize().change_context(ScraperError)?;

                    match processor.add_ers_dca(Box::new(data)).await {
                        Err(e) => {
                            event!(
                                Level::ERROR,
                                "failed to scrape ers_dca for year: {}, err: {:?}",
                                source.year(),
                                e,
                            );
                        }
                        Ok(()) => {
                            event!(
                                Level::INFO,
                                "successfully scraped ers_dca year: {}",
                                source.year()
                            );
                            self.fiskeridir_source
                                .hash_store
                                .add(&hash_id, hash)
                                .await
                                .change_context(ScraperError)?;
                        }
                    };
                }
            }
        }

        Ok(())
    }
}
