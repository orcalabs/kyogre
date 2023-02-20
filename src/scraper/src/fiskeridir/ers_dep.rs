use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::Source;
use kyogre_core::FileHash;
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct ErsDepScraper {
    sources: Vec<Source>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl ErsDepScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, sources: Vec<Source>) -> ErsDepScraper {
        ErsDepScraper {
            sources,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for ErsDepScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDep
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let closure = |ers_dep| processor.add_ers_dep(ers_dep);
        for source in &self.sources {
            if let Err(e) = self
                .fiskeridir_source
                .scrape_year(FileHash::ErsDep, source, closure, 100000)
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to scrape ers_dep for year: {}, err: {:?}",
                    source.year(),
                    e,
                );
            } else {
                event!(
                    Level::INFO,
                    "succesfully scraped ers_dep year: {}",
                    source.year()
                );
            }
        }
        Ok(())
    }
}
