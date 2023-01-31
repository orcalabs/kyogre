use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use kyogre_core::{FileHash, NewLanding};
use tracing::{event, Level};

use super::FiskedirSource;

// Placeholder until the fiskedir-rs crate is done.
pub struct FiskedirLanding;

pub struct LandingScraper {
    min_year: u32,
    max_year: u32,
    fiskedir_source: Arc<FiskedirSource>,
}

impl LandingScraper {
    pub fn new(
        fiskedir_source: Arc<FiskedirSource>,
        min_year: u32,
        max_year: u32,
    ) -> LandingScraper {
        LandingScraper {
            min_year,
            max_year,
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
        for year in self.min_year..=self.max_year {
            if let Err(e) = self
                .fiskedir_source
                .scrape_year::<FiskedirLanding, _, NewLanding, _>(
                    FileHash::Landings,
                    year,
                    closure,
                    1000,
                )
                .await
            {
                event!(
                    Level::ERROR,
                    "failed to scrape landings for year: {year}, err: {:?}",
                    e
                );
            }
        }
        Ok(())
    }
}

impl TryFrom<FiskedirLanding> for NewLanding {
    type Error = ScraperError;

    fn try_from(_value: FiskedirLanding) -> std::result::Result<Self, Self::Error> {
        Ok(NewLanding {})
    }
}
