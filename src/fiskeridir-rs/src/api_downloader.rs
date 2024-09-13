use std::time::Duration;

use http_client::HttpClient;
use serde::{de::DeserializeOwned, Serialize};

use crate::Result;

#[derive(Debug, Clone)]
pub struct ApiDownloader {
    // HTTP client instance
    http_client: HttpClient,
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
    pub fn new() -> Self {
        Self {
            http_client: HttpClient::builder().timeout(Duration::new(60, 0)).build(),
        }
    }

    pub async fn download<T: DeserializeOwned>(
        &self,
        source: &ApiSource,
        query: Option<&impl Serialize>,
    ) -> Result<T> {
        Ok(self
            .http_client
            .download(source.url(), query, None::<String>)
            .await?)
    }
}
