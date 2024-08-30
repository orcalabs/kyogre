use std::sync::Arc;

use async_trait::async_trait;
use error_stack::{Result, ResultExt};
use fiskeridir_rs::{ApiSource, RegisterVessel, RegisterVesselQuery};
use tracing::{error, info};

use crate::{DataSource, FiskeridirSource, Processor, ScraperError, ScraperId};

pub struct RegisterVesselsScraper {
    source: Option<ApiSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl RegisterVesselsScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, source: Option<ApiSource>) -> Self {
        Self {
            fiskeridir_source,
            source,
        }
    }
}

#[async_trait]
impl DataSource for RegisterVesselsScraper {
    fn id(&self) -> ScraperId {
        ScraperId::RegisterVessels
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        if let Some(source) = &self.source {
            let mut register_vessels = Vec::new();
            for i in 1.. {
                let query = RegisterVesselQuery {
                    page: Some(i),
                    per_page: Some(200),
                    ..Default::default()
                };

                let mut vessels: Vec<RegisterVessel> = self
                    .fiskeridir_source
                    .fiskeridir_api
                    .download(source, Some(&query))
                    .await
                    .change_context(ScraperError)
                    .map_err(|e| {
                        error!(
                            "failed to scrape register_vessels for query: {query:?}, err: {e:?}"
                        );
                        e
                    })?;

                match vessels.len() {
                    0 => break,
                    200 => register_vessels.append(&mut vessels),
                    _ => {
                        register_vessels.append(&mut vessels);
                        break;
                    }
                }
            }

            processor
                .add_register_vessels(register_vessels)
                .await
                .change_context(ScraperError)?;

            info!("successfully scraped register_vessels");
        }
        Ok(())
    }
}
