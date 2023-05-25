#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use barentswatch::{FishingFacilityHistoricScraper, FishingFacilityScraper};
use error_stack::Result;
use fiskeridir::{
    ErsDcaScraper, ErsDepScraper, ErsPorScraper, ErsTraScraper, LandingScraper,
    RegisterVesselsScraper, VmsScraper,
};
use kyogre_core::{OauthConfig, ScraperInboundPort, ScraperOutboundPort};
use serde::Deserialize;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{event, instrument, Level};

mod barentswatch;
mod chunks;
mod error;
mod fiskeridir;
mod wrapped_http_client;

pub use barentswatch::BarentswatchSource;
pub use error::*;
pub use fiskeridir::FiskeridirSource;
pub use wrapped_http_client::*;

pub trait Processor: ScraperInboundPort + ScraperOutboundPort + Send + Sync {}
impl<T> Processor for T where T: ScraperInboundPort + ScraperOutboundPort + Send + Sync {}

pub struct Scraper {
    scrapers: Vec<Box<dyn DataSource + Send + Sync>>,
    processor: Box<dyn Processor>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub landings: Option<LandingFileYears>,
    pub ers_dca: Option<Vec<FileYear>>,
    pub ers_por: Option<Vec<FileYear>>,
    pub ers_dep: Option<Vec<FileYear>>,
    pub ers_tra: Option<Vec<FileYear>>,
    pub vms: Option<Vec<FileYear>>,
    pub register_vessels_url: Option<String>,
    pub fishing_facility: Option<ApiClientConfig>,
    pub fishing_facility_historic: Option<ApiClientConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileYear {
    pub year: u32,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LandingFileYears {
    pub min_year: u32,
    pub max_year: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiClientConfig {
    pub url: String,
    #[serde(flatten)]
    pub oauth: Option<OauthConfig>,
}

#[async_trait]
pub trait DataSource: Send + Sync {
    fn id(&self) -> ScraperId;
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError>;
}

impl Scraper {
    pub fn new(
        config: Config,
        processor: Box<dyn Processor>,
        fiskeridir_source: FiskeridirSource,
        barentswatch_source: BarentswatchSource,
    ) -> Scraper {
        let landing_sources = config.landings.map(|landings| {
            (landings.min_year..=landings.max_year)
                .map(|year| fiskeridir_rs::FileSource::Landings { year, url: None })
                .collect()
        });

        let ers_dca_sources = config
            .ers_dca
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::FileSource::ErsDca {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let ers_dep_sources = config
            .ers_dep
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::FileSource::ErsDep {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let ers_por_sources = config
            .ers_por
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::FileSource::ErsPor {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let ers_tra_sources = config
            .ers_tra
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::FileSource::ErsTra {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let vms_sources = config
            .vms
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::FileSource::Vms {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let register_vessels_source = config
            .register_vessels_url
            .map(|url| fiskeridir_rs::ApiSource::RegisterVessels { url });

        let fiskeridir_arc = Arc::new(fiskeridir_source);
        let landings_scraper =
            LandingScraper::new(fiskeridir_arc.clone(), landing_sources.unwrap_or_default());
        let ers_dca_scraper = ErsDcaScraper::new(fiskeridir_arc.clone(), ers_dca_sources);
        let ers_dep_scraper = ErsDepScraper::new(fiskeridir_arc.clone(), ers_dep_sources);
        let ers_por_scraper = ErsPorScraper::new(fiskeridir_arc.clone(), ers_por_sources);
        let ers_tra_scraper = ErsTraScraper::new(fiskeridir_arc.clone(), ers_tra_sources);
        let vms_scraper = VmsScraper::new(fiskeridir_arc.clone(), vms_sources);
        let register_vessels_scraper =
            RegisterVesselsScraper::new(fiskeridir_arc, register_vessels_source);

        let barentswatch_source = Arc::new(barentswatch_source);
        let fishing_facility_scraper =
            FishingFacilityScraper::new(barentswatch_source.clone(), config.fishing_facility);
        let fishing_facility_historic_scraper = FishingFacilityHistoricScraper::new(
            barentswatch_source,
            config.fishing_facility_historic,
        );
        Scraper {
            scrapers: vec![
                Box::new(vms_scraper),
                Box::new(landings_scraper),
                Box::new(ers_dca_scraper),
                Box::new(ers_por_scraper),
                Box::new(ers_dep_scraper),
                Box::new(ers_tra_scraper),
                Box::new(register_vessels_scraper),
                Box::new(fishing_facility_scraper),
                Box::new(fishing_facility_historic_scraper),
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
    ErsPor,
    ErsDep,
    ErsDca,
    ErsTra,
    RegisterVessels,
    Vms,
    FishingFacility,
    FishingFacilityHistoric,
}

impl std::fmt::Display for ScraperId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScraperId::Landings => write!(f, "landings_scraper"),
            ScraperId::ErsPor => write!(f, "ers_por_scraper"),
            ScraperId::ErsDep => write!(f, "ers_dep_scraper"),
            ScraperId::ErsDca => write!(f, "ers_dca_scraper"),
            ScraperId::ErsTra => write!(f, "ers_tra_scraper"),
            ScraperId::RegisterVessels => write!(f, "register_vessels_scraper"),
            ScraperId::Vms => write!(f, "vms_scraper"),
            ScraperId::FishingFacility => write!(f, "fishing_facility_scraper"),
            ScraperId::FishingFacilityHistoric => write!(f, "fishing_facility_historic_scraper"),
        }
    }
}
