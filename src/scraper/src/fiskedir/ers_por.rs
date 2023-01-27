use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;

pub struct ErsPorScraper {
    min_year: u32,
    max_year: u32,
}

impl ErsPorScraper {
    pub fn new(min_year: u32, max_year: u32) -> ErsPorScraper {
        ErsPorScraper { min_year, max_year }
    }
}

#[async_trait]
impl DataSource for ErsPorScraper {
    fn id(&self) -> ScraperId {
        ScraperId::ErsPor
    }

    async fn scrape(&self, _processor: &(dyn Processor)) -> Result<(), ScraperError> {
        unimplemented!();
    }
}
