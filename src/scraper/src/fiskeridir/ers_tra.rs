use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::Source;
use kyogre_core::{FileHash, HashDiff};
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct ErsTraScraper {
    sources: Vec<Source>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl ErsTraScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, sources: Vec<Source>) -> ErsTraScraper {
        ErsTraScraper {
            sources,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for ErsTraScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsTra
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let closure = |ers_tra| processor.add_ers_tra(ers_tra);
        let delete_closure = |year| processor.delete_ers_tra_catches(year);

        for source in &self.sources {
            match self
                .fiskeridir_source
                .scrape_year_if_changed(FileHash::ErsTra, source, closure, 10000, delete_closure)
                .await
            {
                Err(e) => event!(
                    Level::ERROR,
                    "failed to scrape ers_tra for year: {}, err: {:?}",
                    source.year(),
                    e,
                ),
                Ok(HashDiff::Changed) => event!(
                    Level::INFO,
                    "successfully scraped ers_tra year: {}",
                    source.year()
                ),
                Ok(HashDiff::Equal) => event!(
                    Level::INFO,
                    "no changes for ers_tra year: {}",
                    source.year()
                ),
            }
        }
        Ok(())
    }
}
