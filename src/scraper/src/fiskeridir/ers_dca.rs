use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::Source;
use kyogre_core::FileHash;
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct ErsDcaScraper {
    sources: Vec<Source>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl ErsDcaScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, sources: Vec<Source>) -> ErsDcaScraper {
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
        processor
            .delete_ers_dca()
            .await
            .change_context(ScraperError)?;

        let closure = |ers_dca| processor.add_ers_dca(ers_dca);
        for source in &self.sources {
            if let Err(e) = self
                .fiskeridir_source
                .scrape_year(FileHash::ErsDca, source, closure, 100000)
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to scrape ers_dca for year: {}, err: {:?}",
                    source.year(),
                    e,
                );
            } else {
                event!(
                    Level::INFO,
                    "succesfully scraped ers_dca year: {}",
                    source.year()
                );
            }
        }
        Ok(())
    }
}
