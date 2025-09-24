use std::collections::HashMap;

use async_trait::async_trait;
use csv::Reader;
use http_client::HttpClient;
use table_extract::Table;
use tracing::{error, info};

use super::models::DeliveryPoint;
use crate::{DataSource, Processor, Result, ScraperId, error::error::MissingValueSnafu};

pub struct MattilsynetScraper {
    http_client: HttpClient,
    approved_establishments_urls: Vec<String>,
    fishery_products_url: Option<String>,
    fishery_establishements_url: Option<String>,
}

#[async_trait]
impl DataSource for MattilsynetScraper {
    fn id(&self) -> ScraperId {
        ScraperId::Mattilsynet
    }

    async fn scrape(&self, processor: &dyn Processor) -> Result<()> {
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
        fishery_products_url: Option<String>,
        fishery_establishements_url: Option<String>,
    ) -> Self {
        Self {
            http_client: Default::default(),
            approved_establishments_urls: approved_establishments_urls.unwrap_or_default(),
            fishery_products_url,
            fishery_establishements_url,
        }
    }

    async fn do_scrape(&self, processor: &dyn Processor) -> Result<()> {
        let approved = self.approved_establishments().await?;
        let products = self.fishery_products().await?;
        let establishments = self.fishery_establishments().await?;

        let mut delivery_points =
            HashMap::with_capacity(approved.len() + products.len() + establishments.len());

        for d in approved.into_iter().chain(products).chain(establishments) {
            delivery_points
                .entry(d.id.clone())
                .or_insert_with(|| d.into());
        }

        Ok(processor
            .add_mattilsynet_delivery_points(delivery_points.into_values().collect())
            .await?)
    }

    async fn get(&self, url: &str) -> Result<String> {
        Ok(self.http_client.get(url).send().await?.text().await?)
    }

    async fn approved_establishments(&self) -> Result<Vec<DeliveryPoint>> {
        let mut vec = Vec::new();
        for url in &self.approved_establishments_urls {
            let text = self.get(url).await?;

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
                        id: id.parse()?,
                        name: name.to_string(),
                        address: None,
                        postal_code: None,
                        postal_city: None,
                        section: None,
                    });
                }
            }
        }
        Ok(vec)
    }

    async fn fishery_products(&self) -> Result<Vec<DeliveryPoint>> {
        let mut vec = Vec::new();

        if let Some(url) = &self.fishery_products_url {
            let text = self.get(url).await?;

            let reader = Reader::from_reader(text.as_bytes());

            for v in reader.into_deserialize::<DeliveryPoint>() {
                let v = v?;
                if v.section
                    .as_ref()
                    .is_some_and(|v| v.starts_with("Section 8"))
                {
                    vec.push(v);
                }
            }
        }

        Ok(vec)
    }

    async fn fishery_establishments(&self) -> Result<Vec<DeliveryPoint>> {
        if let Some(url) = &self.fishery_establishements_url {
            let text = self.get(url).await?;
            Reader::from_reader(text.as_bytes())
                .into_deserialize::<DeliveryPoint>()
                .collect::<csv::Result<_>>()
                .map_err(|e| e.into())
        } else {
            Ok(vec![])
        }
    }
}
