use crate::error::error::FailedRequestSnafu;
use crate::Result;
use reqwest::{ClientBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ApiDownloader {
    // HTTP client instance
    http_client: reqwest::Client,
}

// Different API sources within Fiskeridir
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiSource {
    RegisterVessels { url: String },
}

impl ApiSource {
    fn url(&self) -> String {
        use ApiSource::*;

        match self {
            RegisterVessels { url } => url.clone(),
        }
    }
}

impl ApiDownloader {
    pub fn new() -> Result<Self> {
        let http_client = ClientBuilder::new().timeout(Duration::new(60, 0)).build()?;

        Ok(Self { http_client })
    }

    pub async fn download<T: DeserializeOwned, Q: Serialize>(
        &self,
        source: &ApiSource,
        query: Option<&Q>,
    ) -> Result<T> {
        let url = source.url();
        let mut request = self.http_client.get(&url);

        if let Some(query) = query {
            request = request.query(&query);
        }

        let response = request.send().await?;
        let status = response.status();
        if status != StatusCode::OK {
            let body = response.text().await?;
            return FailedRequestSnafu { url, status, body }.fail();
        }

        Ok(response.json().await?)
    }
}
