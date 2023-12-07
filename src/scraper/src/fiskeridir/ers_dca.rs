use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::FileSource;
use kyogre_core::{FileHash, FileHashId};
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
            let year = source.year();

            let hash_id = FileHashId::new(FileHash::ErsDca, year);
            let hash = self
                .fiskeridir_source
                .hash_store
                .get_hash(&hash_id)
                .await
                .change_context(ScraperError)?;

            if year < 2020 && hash.is_some() {
                event!(Level::INFO, "skipping ers_dca year: {}", year);
                continue;
            }

            let file = self.fiskeridir_source.download(source).await?;
            let file_hash = file.hash().change_context(ScraperError)?;

            if hash.as_ref() == Some(&file_hash) {
                event!(Level::INFO, "no changes for ers_dca year: {}", year);
            } else {
                let data = file.into_deserialize().change_context(ScraperError)?;

                match processor.add_ers_dca(Box::new(data)).await {
                    Err(e) => {
                        event!(
                            Level::ERROR,
                            "failed to scrape ers_dca for year: {}, err: {:?}",
                            year,
                            e,
                        );
                    }
                    Ok(()) => {
                        event!(Level::INFO, "successfully scraped ers_dca year: {}", year);
                        self.fiskeridir_source
                            .hash_store
                            .add(&hash_id, file_hash)
                            .await
                            .change_context(ScraperError)?;
                    }
                };
            }

            if let Err(e) = self.fiskeridir_source.fiskeridir_file.clean_download_dir() {
                event!(Level::ERROR, "failed to clean download dir: {}", e);
            }
        }

        Ok(())
    }
}
