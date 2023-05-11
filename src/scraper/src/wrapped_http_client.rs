use error_stack::{report, IntoReport, Result, ResultExt};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};

use crate::DownloadError;

pub struct WrappedHttpClient(reqwest::Client);

impl WrappedHttpClient {
    pub fn new() -> reqwest::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::new(60, 0))
            .build()?;

        Ok(Self(client))
    }

    pub async fn download<T: DeserializeOwned, Q: Serialize>(
        &self,
        url: &str,
        query: Option<&Q>,
    ) -> Result<T, DownloadError> {
        let mut request = self.0.get(url);

        if let Some(query) = query {
            request = request.query(&query);
        }

        let response = request
            .send()
            .await
            .into_report()
            .change_context(DownloadError)?;

        if response.status() != StatusCode::OK {
            return Err(report!(DownloadError)
                .attach_printable(format!("received response status {}", response.status())));
        }

        response
            .json()
            .await
            .into_report()
            .change_context(DownloadError)
    }
}
