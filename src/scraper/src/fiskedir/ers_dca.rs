use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;

pub struct ErsDcaScraper {
    min_year: u32,
    max_year: u32,
}

impl ErsDcaScraper {
    pub fn new(min_year: u32, max_year: u32) -> ErsDcaScraper {
        ErsDcaScraper { min_year, max_year }
    }
}

#[async_trait]
impl DataSource for ErsDcaScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsDca
    }

    async fn scrape(&self, _processor: &(dyn Processor)) -> Result<(), ScraperError> {
        unimplemented!();
    }
}
