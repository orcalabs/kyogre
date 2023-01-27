use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;

pub struct ErsDepScraper {
    min_year: u32,
    max_year: u32,
}

impl ErsDepScraper {
    pub fn new(min_year: u32, max_year: u32) -> ErsDepScraper {
        ErsDepScraper { min_year, max_year }
    }
}

#[async_trait]
impl DataSource for ErsDepScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDep
    }

    async fn scrape(&self, _processor: &(dyn Processor)) -> Result<(), ScraperError> {
        unimplemented!();
    }
}
