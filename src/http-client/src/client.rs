use std::time::Duration;

use http::header::AUTHORIZATION;
use reqwest::{Client, IntoUrl};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
use reqwest_tracing::TracingMiddleware;
use serde::{Serialize, de::DeserializeOwned};

use crate::{RequestBuilder, Result};

#[derive(Debug, Clone)]
pub struct HttpClient(ClientWithMiddleware);

#[derive(Default, Debug)]
pub struct HttpClientBuilder {
    client: reqwest::ClientBuilder,
    max_retries: u32,
}

impl HttpClient {
    pub fn new() -> Self {
        Self::new_with(Client::new(), 3)
    }

    fn new_with(inner: Client, max_retries: u32) -> Self {
        let client = ClientBuilder::new(inner)
            .with(TracingMiddleware::default())
            .with(RetryTransientMiddleware::new_with_policy(
                ExponentialBackoff::builder().build_with_max_retries(max_retries),
            ))
            .build();

        Self(client)
    }

    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::new()
    }

    pub fn get(&self, url: impl IntoUrl) -> RequestBuilder {
        RequestBuilder(self.0.get(url))
    }

    pub fn post(&self, url: impl IntoUrl) -> RequestBuilder {
        RequestBuilder(self.0.post(url))
    }

    pub fn put(&self, url: impl IntoUrl) -> RequestBuilder {
        RequestBuilder(self.0.put(url))
    }

    pub fn delete(&self, url: impl IntoUrl) -> RequestBuilder {
        RequestBuilder(self.0.delete(url))
    }

    pub async fn download<T: DeserializeOwned>(
        &self,
        url: impl IntoUrl,
        query: Option<&impl Serialize>,
        token: Option<impl AsRef<str>>,
    ) -> Result<T> {
        let mut req = self.get(url);

        if let Some(query) = query {
            req = req.query(query);
        }
        if let Some(token) = token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token.as_ref()));
        }

        req.send().await?.json().await
    }
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.client = self.client.timeout(timeout);
        self
    }

    pub fn gzip(mut self, enable: bool) -> Self {
        self.client = self.client.gzip(enable);
        self
    }

    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn build(self) -> HttpClient {
        let inner = self.client.build().unwrap();
        HttpClient::new_with(inner, self.max_retries)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
