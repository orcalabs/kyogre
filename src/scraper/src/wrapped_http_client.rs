use crate::{error::error::FailedRequestSnafu, Result};
use kyogre_core::BearerToken;
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};

pub struct WrappedHttpClient(reqwest::Client);

impl WrappedHttpClient {
    pub fn new() -> reqwest::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::new(60, 0))
            .gzip(true)
            .build()?;

        Ok(Self(client))
    }

    pub async fn download<T: DeserializeOwned, Q: Serialize>(
        &self,
        url: &str,
        query: Option<&Q>,
        token: Option<BearerToken>,
    ) -> Result<T> {
        let mut request = self.0.get(url);

        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {}", token.as_ref()));
        }

        if let Some(query) = query {
            request = request.query(&query);
        }

        let response = request.send().await?;
        let status = response.status();

        if status != StatusCode::OK {
            return FailedRequestSnafu {
                url,
                status,
                body: response.text().await?,
            }
            .fail();
        }

        Ok(response.json().await?)
    }
}
