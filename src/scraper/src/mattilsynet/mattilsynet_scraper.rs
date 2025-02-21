use std::collections::HashMap;

use async_trait::async_trait;
use fiskeridir_rs::DeliveryPointId;
use http_client::{HttpClient, Response};
use regex::Regex;
use table_extract::Table;
use tracing::{error, info};

use super::models::DeliveryPoint;
use crate::{DataSource, Processor, Result, ScraperId, error::error::MissingValueSnafu};

/// Statically know location for a file served by Mattilsynet.
pub static MATTILSYNET_FILE_URL_PATH: &str =
    "binary/Virksomheter_som_haandterer_fiskerivarer_-_Fishery_establishments.csv";

pub struct MattilsynetScraper {
    http_client: HttpClient,
    approved_establishments_urls: Vec<String>,
    fishery_establishments_url: Option<String>,
    businesses_url: Option<String>,
}

#[async_trait]
impl DataSource for MattilsynetScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Mattilsynet
    }

    async fn scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        match self.do_scrape(processor).await {
            Ok(()) => {
                info!("successfully scraped mattilsynet delivery points");
                Ok(())
            }
            Err(e) => {
                error!("failed to scrape mattilsynet delivery points, err: {e:?}");
                Err(e)
            }
        }
    }
}

impl MattilsynetScraper {
    pub fn new(
        approved_establishments_urls: Option<Vec<String>>,
        fishery_establishments_url: Option<String>,
        businesses_url: Option<String>,
    ) -> Self {
        Self {
            http_client: Default::default(),
            approved_establishments_urls: approved_establishments_urls.unwrap_or_default(),
            fishery_establishments_url,
            businesses_url,
        }
    }

    async fn do_scrape(&self, processor: &(dyn Processor)) -> Result<()> {
        let approved = self.approved_establishments().await?;
        let fishery = self.fishery_establishments().await?;
        let businesses = self.businesses().await?;

        let mut delivery_points =
            HashMap::with_capacity(approved.len() + fishery.len() + businesses.len());

        for d in approved.into_iter().chain(fishery).chain(businesses) {
            delivery_points
                .entry(d.id.clone())
                .or_insert_with(|| d.into());
        }

        Ok(processor
            .add_mattilsynet_delivery_points(delivery_points.into_values().collect())
            .await?)
    }

    async fn get(&self, url: &str) -> Result<Response> {
        Ok(self.http_client.get(url).send().await?)
    }

    async fn approved_establishments(&self) -> Result<Vec<DeliveryPoint>> {
        let mut vec = Vec::new();
        for url in &self.approved_establishments_urls {
            let response = self.get(url).await?;

            let text = response.text().await?;

            let table = Table::find_first(&text).ok_or_else(|| MissingValueSnafu.build())?;

            for row in table.into_iter() {
                // The id and name are always the two first cell values. We use this assumption instead
                // of getting them through the header key as the table_extract crate returns `None`
                // even if the headers exists.
                let mut iter = row.iter();
                let id = iter.next();
                let name = iter.next();

                if let (Some(id), Some(name)) = (id, name) {
                    vec.push(DeliveryPoint {
                        id: DeliveryPointId::try_from(id.clone())?,
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

    async fn fishery_establishments(&self) -> Result<Vec<DeliveryPoint>> {
        let mut vec = Vec::new();
        if let Some(url) = &self.fishery_establishments_url {
            let mut response = self.get(url).await?;

            if !response.url().path().ends_with(MATTILSYNET_FILE_URL_PATH) {
                response = self
                    .get(&format!("{}/{}", response.url(), MATTILSYNET_FILE_URL_PATH))
                    .await?;
            }

            let text = response.text().await?;

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
                    Some(post_code_stripped.parse::<u32>()?)
                };

                let address = if split[addr_idx].is_empty() {
                    None
                } else {
                    Some(split[addr_idx].to_string())
                };

                vec.push(DeliveryPoint {
                    id: DeliveryPointId::try_from(split[id_idx].to_string())?,
                    name: split[name_idx].to_string(),
                    address,
                    postal_code,
                    postal_city: Some(split[post_city_idx].to_string()),
                })
            }
        }
        Ok(vec)
    }

    async fn businesses(&self) -> Result<Vec<DeliveryPoint>> {
        let mut vec = Vec::new();
        if let Some(url) = &self.businesses_url {
            let text = self.get(url).await?.text().await?;

            let address_code_city = Regex::new(r"(?i)^([^,]+)[, ]+(\d{4})\s+([^,]+)$")?;
            let code_city = Regex::new(r"(?i)^(\d{4})\s+([^,]+)$")?;
            let address_code = Regex::new(r"(?i)^([^,]+)[, ]+(\d{4})$")?;

            for line in text.lines() {
                let mut split = line.split(';');

                let section = split.nth(5).ok_or_else(|| MissingValueSnafu.build())?;

                if !section.contains("General activity establishment") {
                    continue;
                }

                let id = split.next().ok_or_else(|| MissingValueSnafu.build())?;
                let name = split.next().ok_or_else(|| MissingValueSnafu.build())?;
                let address = split
                    .next()
                    .ok_or_else(|| MissingValueSnafu.build())?
                    .trim();

                let (address, postal_code, city) = if let Some((_, [addr, code, city])) =
                    address_code_city.captures(address).map(|c| c.extract())
                {
                    (Some(addr), Some(code), Some(city))
                } else if let Some((_, [code, city])) =
                    code_city.captures(address).map(|c| c.extract())
                {
                    (None, Some(code), Some(city))
                } else if let Some((_, [addr, code])) =
                    address_code.captures(address).map(|c| c.extract())
                {
                    (Some(addr), Some(code), None)
                } else {
                    (None, None, None)
                };

                vec.push(DeliveryPoint {
                    id: id.parse()?,
                    name: name.trim().into(),
                    address: address.map(|v| v.trim().into()),
                    // unwrap is safe because regex only matches if postal_code is 4 digits
                    postal_code: postal_code.map(|v| v.parse().unwrap()),
                    postal_city: city.map(|v| v.trim().into()),
                });
            }
        }
        Ok(vec)
    }
}
