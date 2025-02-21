use reqwest::{
    Body,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::Serialize;

use crate::{Response, Result, error::FailedRequestSnafu};

#[derive(Debug)]
pub struct RequestBuilder(pub(crate) reqwest_middleware::RequestBuilder);

impl RequestBuilder {
    pub fn body(self, body: impl Into<Body>) -> Self {
        Self(self.0.body(body))
    }

    pub fn json(self, json: &impl Serialize) -> Self {
        Self(self.0.json(json))
    }

    pub fn query(self, query: &impl Serialize) -> Self {
        Self(self.0.query(query))
    }

    pub fn header<K, V>(self, key: K, value: V) -> Self
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        Self(self.0.header(key, value))
    }

    pub fn headers(self, headers: HeaderMap) -> Self {
        Self(self.0.headers(headers))
    }

    /// This method will check the status of the response and return an error if it fails
    pub async fn send(self) -> Result<Response> {
        let response = self.0.send().await?;

        let status = response.status();
        if !status.is_success() {
            return FailedRequestSnafu {
                url: response.url().clone(),
                status,
                body: response.text().await?,
            }
            .fail();
        }

        Ok(Response(response))
    }
}
