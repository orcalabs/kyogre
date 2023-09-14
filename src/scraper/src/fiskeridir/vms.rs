use std::sync::Arc;

use crate::{DataSource, Processor, ScraperError, ScraperId};
use async_trait::async_trait;
use error_stack::Result;
use fiskeridir_rs::FileSource;
use kyogre_core::{FileHash, HashDiff};
use tracing::{event, Level};

use super::FiskeridirSource;

pub struct VmsScraper {
    sources: Vec<FileSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl VmsScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, sources: Vec<FileSource>) -> VmsScraper {
        VmsScraper {
            sources,
            fiskeridir_source,
        }
    }
}

#[async_trait]
impl DataSource for VmsScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Vms
    }
    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let closure = |ers_dca| processor.add_vms(ers_dca);

        for source in &self.sources {
            match self
                .fiskeridir_source
                .scrape_year_if_changed(FileHash::Vms, source, closure, 10000)
                .await
            {
                Err(e) => event!(
                    Level::ERROR,
                    "failed to scrape vms for year: {}, err: {:?}",
                    source.year(),
                    e,
                ),
                Ok(HashDiff::Changed) => event!(
                    Level::INFO,
                    "successfully scraped vms year: {}",
                    source.year()
                ),
                Ok(HashDiff::Equal) => {
                    event!(Level::INFO, "no changes for vms year: {}", source.year())
                }
            }
            if let Err(e) = self.fiskeridir_source.fiskeridir_file.clean_download_dir() {
                event!(Level::ERROR, "failed to clean download dir: {}", e);
            }
        }
        Ok(())
    }
}
