use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::FileSource;
use kyogre_core::{FileHash, HashDiff};
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
        let closure = |ers_dca| processor.add_ers_dca(ers_dca);
        let delete_closure = |_| async { Ok(()) };

        for source in &self.sources {
            match self
                .fiskeridir_source
                .scrape_year_if_changed(FileHash::ErsDca, source, closure, 10000, delete_closure)
                .await
            {
                Err(e) => event!(
                    Level::ERROR,
                    "failed to scrape ers_dca for year: {}, err: {:?}",
                    source.year(),
                    e,
                ),
                Ok(HashDiff::Changed) => event!(
                    Level::INFO,
                    "successfully scraped ers_dca year: {}",
                    source.year()
                ),
                Ok(HashDiff::Equal) => event!(
                    Level::INFO,
                    "no changes for ers_dca year: {}",
                    source.year()
                ),
            }
        }
        Ok(())
    }
}
