#![allow(dead_code)]
#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use error_stack::Result;
use fiskedir::LandingScraper;
use kyogre_core::ScraperInboundPort;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{event, instrument, Level};

mod chunks;
mod error;
mod fiskedir;

pub use error::*;
pub use fiskedir::FiskedirSource;

pub trait Processor: ScraperInboundPort + Send + Sync {}
impl<T> Processor for T where T: ScraperInboundPort + Send + Sync {}

pub struct Scraper {
    scrapers: Vec<Box<dyn DataSource + Send + Sync>>,
    processor: Box<dyn Processor>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub landings: FileYear,
    pub ers_dca: FileYear,
    pub ers_por: FileYear,
    pub ers_dep: FileYear,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileYear {
    pub min_year: u32,
    pub max_year: u32,
}

#[async_trait]
pub trait DataSource: Send + Sync {
    fn id(&self) -> ScraperId;
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError>;
}

impl Scraper {
    pub fn new(config: Config, processor: Box<dyn Processor>, source: FiskedirSource) -> Scraper {
        let arc = Arc::new(source);
        let landings_scraper =
            LandingScraper::new(arc, config.landings.min_year, config.landings.min_year);
        Scraper {
            scrapers: vec![Box::new(landings_scraper)],
            processor,
        }
    }

    pub async fn run(&self) {
        for s in &self.scrapers {
            self.run_scraper(s.as_ref()).await;
        }
    }

    #[instrument(skip_all, fields(app.scraper))]
    async fn run_scraper(&self, s: &(dyn DataSource)) {
        tracing::Span::current().record("app.scraper", s.id().to_string());
        if let Err(e) = s.scrape(self.processor.as_ref()).await {
            event!(Level::ERROR, "failed to run scraper: {:?}", e);
        }
    }
}

pub enum ScraperId {
    Landings,
    /// All existing landing ids (Fiskedirektoratet can remove landings upon request)
    LandingIds,
    ErsPor,
    ErsDep,
    ErsDca,
}

impl std::fmt::Display for ScraperId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScraperId::Landings => write!(f, "landings_scraper"),
            ScraperId::LandingIds => write!(f, "landing_ids_scraper"),
            ScraperId::ErsPor => write!(f, "ers_por_scraper"),
            ScraperId::ErsDep => write!(f, "ers_dep_scraper"),
            ScraperId::ErsDca => write!(f, "ers_dca_scraper"),
        }
    }
}
