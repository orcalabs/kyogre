use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;

use crate::Result;

#[derive(Debug)]
pub struct Response(pub(crate) reqwest::Response);

impl Response {
    pub fn status(&self) -> StatusCode {
        self.0.status()
    }

    pub fn url(&self) -> &Url {
        self.0.url()
    }

    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        self.0.json().await.map_err(From::from)
    }

    pub async fn text(self) -> Result<String> {
        self.0.text().await.map_err(From::from)
    }

    pub async fn bytes(self) -> Result<Bytes> {
        self.0.bytes().await.map_err(From::from)
    }

    pub fn bytes_stream(self) -> impl Stream<Item = Result<Bytes>> {
        self.0.bytes_stream().map_err(From::from)
    }
}
