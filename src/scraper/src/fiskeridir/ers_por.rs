use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::Source;
use kyogre_core::FileHash;
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct ErsPorScraper {
    sources: Vec<Source>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl ErsPorScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, sources: Vec<Source>) -> ErsPorScraper {
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
            if let Err(e) = self
                .fiskeridir_source
                .scrape_year(FileHash::ErsPor, source, closure, 10000)
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to scrape ers_por for year: {}, err: {:?}",
                    source.year(),
                    e,
                );
            } else {
                event!(
                    Level::INFO,
                    "succesfully scraped ers_por year: {}",
                    source.year()
                );
            }
        }
        Ok(())
    }
}
