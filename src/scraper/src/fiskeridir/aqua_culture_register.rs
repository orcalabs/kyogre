use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::FileSource;
use kyogre_core::{FileHash, HashDiff};
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct AquaCultureRegisterScraper {
    source: Option<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl AquaCultureRegisterScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        source: Option<FileSource>,
    ) -> AquaCultureRegisterScraper {
        AquaCultureRegisterScraper {
            source,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for AquaCultureRegisterScraper {
    fn id(&self) -> ScraperId {
        ScraperId::AquaCultureRegister
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let closure = |data| processor.add_aqua_culture_register(data);

        if let Some(source) = &self.source {
            match self
                .fiskeridir_source
                .scrape_year_if_changed(FileHash::AquaCultureRegister, source, closure, 10000)
                .await
            {
                Err(e) => event!(
                    Level::ERROR,
                    "failed to scrape aqua_culture_register for year: {}, err: {:?}",
                    source.year(),
                    e,
                ),
                Ok(HashDiff::Changed) => event!(
                    Level::INFO,
                    "successfully scraped aqua_culture_register year: {}",
                    source.year()
                ),
                Ok(HashDiff::Equal) => event!(
                    Level::INFO,
                    "no changes for aqua_culture_register year: {}",
                    source.year()
                ),
            }
        }
        Ok(())
    }
}
