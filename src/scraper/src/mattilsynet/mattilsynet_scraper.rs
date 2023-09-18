use std::collections::HashMap;

use async_trait::async_trait;
use error_stack::{report, Result, ResultExt};
use fiskeridir_rs::DeliveryPointId;
use reqwest::{Response, StatusCode};
use table_extract::Table;
use tracing::{event, Level};

use crate::{DataSource, Processor, ScraperError, ScraperId};

use super::models::DeliveryPoint;

/// Statically know location for a file served by Mattilsynet.
pub static MATTILSYNET_FILE_URL_PATH: &str =
    "binary/Virksomheter_som_haandterer_fiskerivarer_-_Fishery_establishments.csv";

pub struct MattilsynetScraper {
    http_client: reqwest::Client,
    approved_establishments_urls: Vec<String>,
    fishery_establishments_url: Option<String>,
}

#[async_trait]
impl DataSource for MattilsynetScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Mattilsynet
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        match self.do_scrape(processor).await {
            Ok(()) => {
                event!(
                    Level::INFO,
                    "successfully scraped mattilsynet delivery points",
                );
                Ok(())
            }
            Err(e) => {
                event!(
                    Level::ERROR,
                    "failed to scrape mattilsynet delivery points, err: {:?}",
                    e
                );
                Err(e)
            }
        }
    }
}

impl MattilsynetScraper {
    pub fn new(
        approved_establishments_urls: Option<Vec<String>>,
        fishery_establishments_url: Option<String>,
    ) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            approved_establishments_urls: approved_establishments_urls.unwrap_or_default(),
            fishery_establishments_url,
        }
    }

    async fn do_scrape(&self, processor: &(dyn Processor)) -> Result<(), ScraperError> {
        let approved = self.approved_establishments().await?;
        let fishery = self.fishery_establishments().await?;

        let mut delivery_points = HashMap::with_capacity(approved.len() + fishery.len());

        for d in approved.into_iter().chain(fishery) {
            delivery_points
                .entry(d.id.clone())
                .or_insert_with(|| d.into());
        }

        processor
            .add_mattilsynet_delivery_points(delivery_points.into_values().collect())
            .await
            .change_context(ScraperError)
    }

    async fn get(&self, url: &str) -> Result<Response, ScraperError> {
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .change_context(ScraperError)?;

        let status = response.status();

        match status {
            StatusCode::OK => Ok(response),
            _ => Err(report!(Error::Download {
                status,
                text: response.text().await.change_context(ScraperError)?
            })
            .change_context(ScraperError)),
        }
    }

    async fn approved_establishments(&self) -> Result<Vec<DeliveryPoint>, ScraperError> {
        let mut vec = Vec::new();
        for url in &self.approved_establishments_urls {
            let response = self.get(url).await?;

            let text = response.text().await.change_context(ScraperError)?;

            let table = Table::find_first(&text)
                .ok_or_else(|| report!(Error::MissingTable).change_context(ScraperError))?;

            for row in table.into_iter() {
                // The id and name are always the two first cell values. We use this assumption instead
                // of getting them through the header key as the table_extract crate returns `None`
                // even if the headers exists.
                let mut iter = row.iter();
                let id = iter.next();
                let name = iter.next();

                if let (Some(id), Some(name)) = (id, name) {
                    vec.push(DeliveryPoint {
                        id: DeliveryPointId::try_from(id.clone()).change_context(ScraperError)?,
                        name: name.to_string(),
                        address: None,
                        postal_code: None,
                        postal_city: None,
                    });
                }
            }
        }
        Ok(vec)
    }

    async fn fishery_establishments(&self) -> Result<Vec<DeliveryPoint>, ScraperError> {
        let mut vec = Vec::new();
        if let Some(url) = &self.fishery_establishments_url {
            let mut response = self.get(url).await?;

            if !response.url().path().ends_with(MATTILSYNET_FILE_URL_PATH) {
                response = self
                    .get(&format!("{}/{}", response.url(), MATTILSYNET_FILE_URL_PATH))
                    .await?;
            }

            let text = response.text().await.change_context(ScraperError)?;

            for l in text.lines().skip(10) {
                if l.starts_with(";;;") || l.starts_with(";Approval") || l.starts_with("Approval") {
                    continue;
                }

                let (id_idx, name_idx, addr_idx, post_code_idx, post_city_idx) =
                    if l.starts_with(';') {
                        (1, 2, 3, 7, 8)
                    } else {
                        (0, 1, 2, 5, 6)
                    };

                let split = l.split(';').collect::<Vec<_>>();

                let post_code_stripped = split[post_code_idx].replace(' ', "");
                let postal_code: Option<u32> = if post_code_stripped.is_empty() {
                    None
                } else {
                    Some(
                        post_code_stripped
                            .parse::<u32>()
                            .change_context(ScraperError)
                            .attach_printable_lazy(|| {
                                format!("could not parse '{post_code_stripped}' as u32")
                            })?,
                    )
                };

                let address = if split[addr_idx].is_empty() {
                    None
                } else {
                    Some(split[addr_idx].to_string())
                };

                vec.push(DeliveryPoint {
                    id: DeliveryPointId::try_from(split[id_idx].to_string())
                        .change_context(ScraperError)?,
                    name: split[name_idx].to_string(),
                    address,
                    postal_code,
                    postal_city: Some(split[post_city_idx].to_string()),
                })
            }
        }
        Ok(vec)
    }
}

#[derive(Debug)]
pub enum Error {
    Download { status: StatusCode, text: String },
    MissingTable,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Download { status, text } => f.write_str(&format!(
                "error downloading data from source, status: {status}, text: {text}",
            )),
            Error::MissingTable => f.write_str("`Table::find_first` returned `None`"),
        }
    }
}
