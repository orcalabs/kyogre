#![deny(warnings)]
#![deny(rust_2018_idioms)]

use async_trait::async_trait;
use barentswatch::{FishingFacilityHistoricScraper, FishingFacilityScraper};
use fiskeridir::{
    AquaCultureRegisterScraper, ErsScraper, LandingScraper, RegisterVesselsScraper, VmsScraper,
};
use kyogre_core::{OauthConfig, ScraperInboundPort, ScraperOutboundPort};
use mattilsynet::MattilsynetScraper;
use ocean_climate::OceanClimateScraper;
use orca_core::Environment;
use serde::Deserialize;
use std::sync::Arc;
use std::{fmt::Debug, path::PathBuf};
use tracing::{error, instrument};
use weather::WeatherScraper;

mod barentswatch;
mod chunks;
mod error;
mod fiskeridir;
mod mattilsynet;
mod ocean_climate;
mod utils;
mod weather;

pub use barentswatch::BarentswatchSource;
pub use error::{Error, Result};
pub use fiskeridir::FiskeridirSource;

pub trait Processor: ScraperInboundPort + ScraperOutboundPort + Send + Sync {}
impl<T> Processor for T where T: ScraperInboundPort + ScraperOutboundPort + Send + Sync {}

pub struct Scraper {
    environment: Environment,
    scrapers: Vec<Vec<Arc<dyn DataSource + Send + Sync>>>,
    processor: Arc<dyn Processor>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub landings: Option<FileYears>,
    pub ers: Option<FileYears>,
    pub vms: Option<Vec<FileYear>>,
    pub aqua_culture_register_url: Option<String>,
    pub mattilsynet_urls: Option<Vec<String>>,
    pub mattilsynet_fishery_url: Option<String>,
    pub mattilsynet_businesses_url: Option<String>,
    pub register_vessels_url: Option<String>,
    pub fishing_facility: Option<ApiClientConfig>,
    pub fishing_facility_historic: Option<ApiClientConfig>,
    pub file_download_dir: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileYear {
    pub year: u32,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileYears {
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
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()>;
}

impl Scraper {
    pub fn new(
        environment: Environment,
        config: Config,
        processor: Arc<dyn Processor>,
        fiskeridir_source: FiskeridirSource,
        barentswatch_source: BarentswatchSource,
    ) -> Scraper {
        let landing_sources = config
            .landings
            .map(|landings| {
                (landings.min_year..=landings.max_year)
                    .map(|year| fiskeridir_rs::FileSource::Landings { year, url: None })
                    .collect()
            })
            .unwrap_or_default();

        let ers_sources = config
            .ers
            .map(|ers| {
                (ers.min_year..=ers.max_year)
                    .map(|year| fiskeridir_rs::FileSource::Ers { year, url: None })
                    .collect()
            })
            .unwrap_or_default();

        let vms_sources = config
            .vms
            .unwrap_or_default()
            .into_iter()
            .map(|file_year| fiskeridir_rs::FileSource::Vms {
                year: file_year.year,
                url: file_year.url,
            })
            .collect();

        let aqua_culture_register_source = config
            .aqua_culture_register_url
            .map(|url| fiskeridir_rs::FileSource::AquaCultureRegister { url });

        let register_vessels_source = config
            .register_vessels_url
            .map(|url| fiskeridir_rs::ApiSource::RegisterVessels { url });

        let fiskeridir_arc = Arc::new(fiskeridir_source);
        let landings_scraper =
            LandingScraper::new(fiskeridir_arc.clone(), landing_sources, environment);
        let ers_scraper = ErsScraper::new(fiskeridir_arc.clone(), ers_sources, environment);
        let vms_scraper = VmsScraper::new(fiskeridir_arc.clone(), vms_sources, environment);
        let aqua_culture_register_scraper = AquaCultureRegisterScraper::new(
            fiskeridir_arc.clone(),
            aqua_culture_register_source,
            environment,
        );
        let mattilsynet_scraper = MattilsynetScraper::new(
            config.mattilsynet_urls,
            config.mattilsynet_fishery_url,
            config.mattilsynet_businesses_url,
        );
        let register_vessels_scraper =
            RegisterVesselsScraper::new(fiskeridir_arc, register_vessels_source);

        let barentswatch_source = Arc::new(barentswatch_source);
        let fishing_facility_scraper =
            FishingFacilityScraper::new(barentswatch_source.clone(), config.fishing_facility);
        let fishing_facility_historic_scraper = FishingFacilityHistoricScraper::new(
            barentswatch_source,
            config.fishing_facility_historic,
        );

        let weather_scraper = WeatherScraper::new();
        let _ocean_climate_scraper = OceanClimateScraper::new();

        Scraper {
            environment,
            scrapers: vec![
                vec![
                    Arc::new(landings_scraper),
                    Arc::new(register_vessels_scraper),
                    Arc::new(ers_scraper),
                    Arc::new(fishing_facility_scraper),
                    Arc::new(fishing_facility_historic_scraper),
                    Arc::new(aqua_culture_register_scraper),
                ],
                vec![Arc::new(vms_scraper)],
                vec![Arc::new(mattilsynet_scraper)],
                vec![Arc::new(weather_scraper)],
                // vec![Box::new(ocean_climate_scraper)],
            ],
            processor,
        }
    }
}

#[instrument(skip_all, fields(app.scraper))]
async fn run_scraper(s: &(dyn DataSource), processor: &(dyn Processor)) {
    tracing::Span::current().record("app.scraper", s.id().to_string());
    if let Err(e) = s.scrape(processor).await {
        error!("failed to run scraper: {e:?}");
    }
}

#[async_trait]
impl kyogre_core::Scraper for Scraper {
    async fn run(&self) {
        match self.environment {
            Environment::Local => {
                let handles = self
                    .scrapers
                    .iter()
                    .map(|scrapers| {
                        let scrapers = scrapers.clone();
                        let processor = self.processor.clone();
                        tokio::spawn(async move {
                            for s in scrapers {
                                run_scraper(s.as_ref(), processor.as_ref()).await;
                            }
                        })
                    })
                    .collect::<Vec<_>>();

                for h in handles {
                    if let Err(e) = h.await {
                        error!("failed to run scraper: {e:?}");
                    }
                }
            }
            Environment::Production
            | Environment::OnPremise
            | Environment::Development
            | Environment::Test => {
                for ss in &self.scrapers {
                    for s in ss {
                        run_scraper(s.as_ref(), self.processor.as_ref()).await;
                    }
                }
            }
        }
    }
}

pub enum ScraperId {
    Landings,
    Ers,
    RegisterVessels,
    Vms,
    FishingFacility,
    FishingFacilityHistoric,
    AquaCultureRegister,
    Mattilsynet,
    Weather,
    OceanClimate,
}

impl std::fmt::Display for ScraperId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScraperId::Landings => write!(f, "landings_scraper"),
            ScraperId::Ers => write!(f, "ers_scraper"),
            ScraperId::RegisterVessels => write!(f, "register_vessels_scraper"),
            ScraperId::Vms => write!(f, "vms_scraper"),
            ScraperId::FishingFacility => write!(f, "fishing_facility_scraper"),
            ScraperId::FishingFacilityHistoric => write!(f, "fishing_facility_historic_scraper"),
            ScraperId::AquaCultureRegister => write!(f, "aqua_culture_register"),
            ScraperId::Mattilsynet => write!(f, "mattilsynet"),
            ScraperId::Weather => write!(f, "weather"),
            ScraperId::OceanClimate => write!(f, "ocean_climate"),
        }
    }
}
