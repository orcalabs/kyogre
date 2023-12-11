use std::time::Duration;

use error_stack::ResultExt;
use error_stack::{report, Result};
use reqwest::{ClientBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::Error;

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
    pub fn new() -> Result<Self, Error> {
        let http_client = ClientBuilder::new()
            .timeout(Duration::new(60, 0))
            .build()
            .change_context(Error::Download)?;

        Ok(Self { http_client })
    }

    pub async fn download<T: DeserializeOwned, Q: Serialize>(
        &self,
        source: &ApiSource,
        query: Option<&Q>,
    ) -> Result<T, Error> {
        let mut request = self.http_client.get(source.url());

        if let Some(query) = query {
            request = request.query(&query);
        }

        let response = request.send().await.change_context(Error::Download)?;

        if response.status() != StatusCode::OK {
            return Err(report!(Error::Download)
                .attach_printable(format!("received response status {}", response.status())));
        }

        response.json().await.change_context(Error::Deserialize)
    }
}
