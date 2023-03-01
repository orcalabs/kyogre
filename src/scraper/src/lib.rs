#![allow(dead_code)]
#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use error_stack::Result;
use fiskeridir::{ErsDcaScraper, ErsDepScraper, ErsPorScraper, ErsTraScraper, LandingScraper};
use kyogre_core::ScraperInboundPort;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{event, instrument, Level};

mod chunks;
mod error;
mod fiskeridir;

pub use error::*;
pub use fiskeridir::FiskeridirSource;

pub trait Processor: ScraperInboundPort + Send + Sync {}
impl<T> Processor for T where T: ScraperInboundPort + Send + Sync {}

pub struct Scraper {
    scrapers: Vec<Box<dyn DataSource + Send + Sync>>,
    processor: Box<dyn Processor>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub landings: LandingFileYears,
    pub ers_dca: Option<Vec<ErsFileYear>>,
    pub ers_por: Option<Vec<ErsFileYear>>,
    pub ers_dep: Option<Vec<ErsFileYear>>,
    pub ers_tra: Option<Vec<ErsFileYear>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ErsFileYear {
    pub year: u32,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LandingFileYears {
    pub min_year: u32,
    pub max_year: u32,
}

#[async_trait]
pub trait DataSource: Send + Sync {
    fn id(&self) -> ScraperId;
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError>;
}

impl Scraper {
    pub fn new(config: Config, processor: Box<dyn Processor>, source: FiskeridirSource) -> Scraper {
        let landing_sources = (config.landings.min_year..=config.landings.max_year)
            .map(|year| fiskeridir_rs::Source::Landings { year, url: None })
            .collect();

        let ers_dca_sources = config
            .ers_dca
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::Source::ErsDca {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let ers_dep_sources = config
            .ers_dep
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::Source::ErsDep {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let ers_por_sources = config
            .ers_por
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::Source::ErsPor {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let ers_tra_sources = config
            .ers_tra
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::Source::ErsTra {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let arc = Arc::new(source);
        let _landings_scraper = LandingScraper::new(arc.clone(), landing_sources);
        let ers_dca_scraper = ErsDcaScraper::new(arc.clone(), ers_dca_sources);
        let ers_dep_scraper = ErsDepScraper::new(arc.clone(), ers_dep_sources);
        let ers_por_scraper = ErsPorScraper::new(arc.clone(), ers_por_sources);
        let ers_tra_scraper = ErsTraScraper::new(arc, ers_tra_sources);
        Scraper {
            scrapers: vec![
                // Box::new(landings_scraper),
                Box::new(ers_dca_scraper),
                Box::new(ers_dep_scraper),
                Box::new(ers_por_scraper),
                Box::new(ers_tra_scraper),
            ],
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
    /// All existing landing ids (Fiskeridirektoratet can remove landings upon request)
    LandingIds,
    ErsPor,
    ErsDep,
    ErsDca,
    ErsTra,
}

impl std::fmt::Display for ScraperId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScraperId::Landings => write!(f, "landings_scraper"),
            ScraperId::LandingIds => write!(f, "landing_ids_scraper"),
            ScraperId::ErsPor => write!(f, "ers_por_scraper"),
            ScraperId::ErsDep => write!(f, "ers_dep_scraper"),
            ScraperId::ErsDca => write!(f, "ers_dca_scraper"),
            ScraperId::ErsTra => write!(f, "ers_tra_scraper"),
        }
    }
}
