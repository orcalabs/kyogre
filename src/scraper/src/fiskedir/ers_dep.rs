use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use kyogre_core::NewDeparture;

use super::FiskedirSource;

// Placeholder until the fiskedir-rs crate is done.
pub struct FiskedirDep;

pub struct ErsDepScraper {
    min_year: u32,
    max_year: u32,
    fiskedir_source: Arc<FiskedirSource>,
}

impl ErsDepScraper {
    pub fn new(
        fiskedir_source: Arc<FiskedirSource>,
        min_year: u32,
        max_year: u32,
    ) -> ErsDepScraper {
        ErsDepScraper {
            min_year,
            max_year,
            fiskedir_source,
        }
    }
}

#[async_trait]
impl DataSource for ErsDepScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDep
    }

    async fn scrape(&self, _processor: &(dyn Processor)) -> Result<(), ScraperError> {
        unimplemented!();
        // let closure = |landings| processor.add_departure(landings);
        // for year in self.min_year..=self.max_year {
        //     if let Err(e) = self
        //         .fiskedir_source
        //         .scrape_year::<FiskedirDep, _, NewDeparture, _>(
        //             FileHash::Landings,
        //             year,
        //             closure,
        //             1000,
        //         )
        //         .await
        //     {
        //         event!(
        //             Level::ERROR,
        //             "failed to scrape dep for year: {year}, err: {:?}",
        //             e
        //         );
        //     }
        // }
        // Ok(())
    }
}

impl TryFrom<FiskedirDep> for NewDeparture {
    type Error = ScraperError;

    fn try_from(_value: FiskedirDep) -> std::result::Result<Self, Self::Error> {
        Ok(NewDeparture {})
    }
}
