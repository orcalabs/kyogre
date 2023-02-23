use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::Source;
use kyogre_core::FileHash;
use tracing::{event, Level};

use super::FiskedirSource;

pub struct LandingScraper {
    sources: Vec<Source>,
    fiskedir_source: Arc<FiskedirSource>,
}

impl LandingScraper {
    pub fn new(fiskedir_source: Arc<FiskedirSource>, sources: Vec<Source>) -> LandingScraper {
        LandingScraper {
            sources,
            fiskedir_source,
        }
    }
}

#[async_trait]
impl DataSource for LandingScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Landings
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let closure = |landings| processor.add_landings(landings);
        for source in &self.sources {
            if let Err(e) = self
                .fiskedir_source
                .scrape_year::<fiskeridir_rs::LandingRaw, fiskeridir_rs::Landing, _, _>(
                    FileHash::Landings,
                    source,
                    closure,
                    1000,
                )
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to scrape landings for year: {}, err: {:?}",
                    source.year(),
                    e,
                );
            } else {
                event!(
                    Level::INFO,
                    "succesfully scraped landings year: {}",
                    source.year()
                );
            }
        }
        Ok(())
    }
}
