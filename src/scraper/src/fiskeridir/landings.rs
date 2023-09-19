use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{FileSource, Landing, LandingRaw};
use kyogre_core::{FileHash, FileHashId};
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
            let hash_id = FileHashId::new(FileHash::Landings, source.year());
            let hash = self
                .fiskeridir_source
                .hash_store
                .get_hash(&hash_id)
                .await
                .change_context(ScraperError)?;

            if source.year() < 2020 && hash.is_some() {
                event!(Level::INFO, "skipping landings year: {}", source.year());
                continue;
            }

            let file = self.fiskeridir_source.download(source).await?;
            let file_hash = file.hash().change_context(ScraperError)?;

            if hash.as_ref() == Some(&file_hash) {
                event!(
                    Level::INFO,
                    "no changes for landings year: {}",
                    source.year()
                );
            } else {
                let data = file
                    .into_deserialize::<LandingRaw>()
                    .change_context(ScraperError)?
                    .map(|v| match v {
                        Ok(v) => Landing::try_from(v),
                        Err(e) => Err(e),
                    });

                match processor.add_landings(Box::new(data), source.year()).await {
                    Err(e) => {
                        event!(
                            Level::ERROR,
                            "failed to scrape landings for year: {}, err: {:?}",
                            source.year(),
                            e,
                        );
                    }
                    Ok(()) => {
                        event!(
                            Level::INFO,
                            "successfully scraped landings year: {}",
                            source.year()
                        );
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
