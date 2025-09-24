use async_trait::async_trait;
use chrono::NaiveDateTime;
use fiskeridir_rs::{ApiSource, BuyerLocation, BuyerLocationsQuery, BuyerSortField};
use std::sync::Arc;
use tracing::{error, info};

use crate::{DataSource, FiskeridirSource, Processor, Result, ScraperId};

pub struct BuyerRegisterScraper {
    source: Option<ApiSource>,
    fiskeridir_source: Arc<FiskeridirSource>,
}

impl BuyerRegisterScraper {
    pub fn new(fiskeridir_source: Arc<FiskeridirSource>, source: Option<ApiSource>) -> Self {
        Self {
            fiskeridir_source,
            source,
        }
    }
}

#[async_trait]
impl DataSource for BuyerRegisterScraper {
    fn id(&self) -> ScraperId {
        ScraperId::BuyerRegister
    }

    async fn scrape(&self, processor: &dyn Processor) -> Result<()> {
        if let Some(source) = &self.source {
            let latest = processor
                .latest_buyer_location_update()
                .await?
                .unwrap_or(NaiveDateTime::MIN);

            let limit = 1_000;
            let mut locations = Vec::new();

            for i in 1.. {
                let query = BuyerLocationsQuery {
                    page: Some(i),
                    per_page: Some(limit),
                    sort_asc: Some(true),
                    sort_by: Some(BuyerSortField::LocationId),
                    ..Default::default()
                };

                let locs: Vec<BuyerLocation> = self
                    .fiskeridir_source
                    .fiskeridir_api
                    .download(source, Some(&query))
                    .await
                    .inspect_err(|e| {
                        error!("failed to scrape buyer locations for query: {query:?}, err: {e:?}");
                    })?;

                match locs.len() {
                    0 => break,
                    v => {
                        locations.extend(
                            locs.into_iter()
                                .filter(|v| v.updated > latest)
                                .filter_map(|v| match kyogre_core::BuyerLocation::try_from(v) {
                                    Ok(v) => Some(v),
                                    Err(e) => {
                                        error!("failed to convert value to BuyerLocation: {e:?}");
                                        None
                                    }
                                }),
                        );

                        if v != limit as usize {
                            break;
                        }
                    }
                }
            }

            if !locations.is_empty() {
                processor.add_buyer_locations(locations).await?;
            }

            info!("successfully scraped buyer_register");
        }
        Ok(())
    }
}
