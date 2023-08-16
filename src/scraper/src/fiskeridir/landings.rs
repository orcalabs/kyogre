use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{FileSource, Landing, LandingRaw};
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
                HashDiff::Equal => event!(
                    Level::INFO,
                    "no changes for landings year: {}",
                    source.year()
                ),
                HashDiff::Changed => {
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
