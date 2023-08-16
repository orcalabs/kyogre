use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::FileSource;
use kyogre_core::{FileHash, HashDiff};
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct ErsPorScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl ErsPorScraper {
    pub fn new(
        fiskeridir_source: Arc<FiskeridirSource>,
        sources: Vec<FileSource>,
    ) -> ErsPorScraper {
        ErsPorScraper {
            sources,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for ErsPorScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsPor
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let closure = |ers_por| processor.add_ers_por(ers_por);

        for source in &self.sources {
            match self
                .fiskeridir_source
                .scrape_year_if_changed(FileHash::ErsPor, source, closure, 10000)
                .await
            {
                Err(e) => event!(
                    Level::ERROR,
                    "failed to scrape ers_por for year: {}, err: {:?}",
                    source.year(),
                    e,
                ),
                Ok(HashDiff::Changed) => event!(
                    Level::INFO,
                    "successfully scraped ers_por year: {}",
                    source.year()
                ),
                Ok(HashDiff::Equal) => event!(
                    Level::INFO,
                    "no changes for ers_por year: {}",
                    source.year()
                ),
            }
        }
        Ok(())
    }
}
