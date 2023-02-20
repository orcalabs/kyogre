use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use kyogre_core::NewArrival;

use super::FiskedirSource;

// Placeholder until the fiskedir-rs crate is done.
pub struct FiskedirPor;

pub struct ErsPorScraper {
    min_year: u32,
    max_year: u32,
    fiskedir_source: Arc<FiskedirSource>,
}

impl ErsPorScraper {
    pub fn new(
        fiskedir_source: Arc<FiskedirSource>,
        min_year: u32,
        max_year: u32,
    ) -> ErsPorScraper {
        ErsPorScraper {
            min_year,
            max_year,
            fiskedir_source,
        }
    }
}

#[async_trait]
impl DataSource for ErsPorScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDca
    }

    async fn scrape(&self, _processor: &(dyn Processor)) -> Result<(), ScraperError> {
        unimplemented!();
        // let closure = |landings| processor.add_arrival(landings);
        // for year in self.min_year..=self.max_year {
        //     if let Err(e) = self
        //         .fiskedir_source
        //         .scrape_year::<FiskedirPor, _, NewArrival, _>(
        //             FileHash::Landings,
        //             year,
        //             closure,
        //             1000,
        //         )
        //         .await
        //     {
        //         event!(
        //             Level::ERROR,
        //             "failed to scrape por for year: {year}, err: {:?}",
        //             e
        //         );
        //     }
        // }
        // Ok(())
    }
}

impl TryFrom<FiskedirPor> for NewArrival {
    type Error = ScraperError;

    fn try_from(_value: FiskedirPor) -> std::result::Result<Self, Self::Error> {
        Ok(NewArrival {})
    }
}
