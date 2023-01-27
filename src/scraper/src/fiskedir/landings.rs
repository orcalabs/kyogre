use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;

pub struct LandingScraper {
    min_year: u32,
    max_year: u32,
}

impl LandingScraper {
    pub fn new(min_year: u32, max_year: u32) -> LandingScraper {
        LandingScraper { min_year, max_year }
    }
}

#[async_trait]
impl DataSource for LandingScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Landings
    }

    async fn scrape(&self, _processor: &(dyn Processor)) -> Result<(), ScraperError> {
        Ok(())
    }
}
