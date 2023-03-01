use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::Source;
use kyogre_core::FileHash;
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
        for source in &self.sources {
            if let Err(e) = self
                .fiskeridir_source
                .scrape_year(FileHash::ErsTra, source, closure, 10000)
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to scrape ers_tra for year: {}, err: {:?}",
                    source.year(),
                    e,
                );
            } else {
                event!(
                    Level::INFO,
                    "succesfully scraped ers_tra year: {}",
                    source.year()
                );
            }
        }
        Ok(())
    }
}
