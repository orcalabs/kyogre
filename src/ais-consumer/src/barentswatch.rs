use crate::error::BarentswatchClientError;
use error_stack::{bail, Result, ResultExt};

use futures::{StreamExt, TryStreamExt};
use kyogre_core::BearerToken;
use reqwest::{Client, Url};
use serde::Serialize;
use tokio::io::AsyncRead;

pub struct BarentswatchAisClient {
    bearer_token: BearerToken,
    api_address: Url,
    client: Client,
}

#[derive(Serialize)]
struct AisFilterArgs {
    downsample: bool,
    #[serde(rename = "includePosition")]
    include_position: bool,
    #[serde(rename = "includeStatic")]
    include_static: bool,
    #[serde(rename = "includeAton")]
    include_aton: bool,
    #[serde(rename = "includeSafetyRelated")]
    include_safety_related: bool,
    #[serde(rename = "includeBinaryBroadcastMetHyd")]
    include_binary_broadcast: bool,
}

impl BarentswatchAisClient {
    pub fn new(bearer_token: BearerToken, api_address: Url) -> BarentswatchAisClient {
        BarentswatchAisClient {
            bearer_token,
            api_address,
            client: Client::new(),
        }
    }

    /// Returns the ais source as stream which will continuously receive data from the source.
    pub async fn streamer(&self) -> Result<impl AsyncRead, BarentswatchClientError> {
        let args = AisFilterArgs {
            downsample: true,
            include_position: true,
            include_static: true,
            include_aton: false,
            include_safety_related: false,
            include_binary_broadcast: false,
        };

        let response = self
            .client
            .post(self.api_address.clone())
            .json(&args)
            .header(
                "Authorization",
                format!("bearer {}", self.bearer_token.as_ref()),
            )
            .header("Content-type", "application/json")
            .send()
            .await
            .change_context(BarentswatchClientError::SendingRequest)?;

        let status = response.status();
        if !status.is_success() {
            bail!(BarentswatchClientError::Server {
                response_code: status.as_u16(),
                body: response
                    .text()
                    .await
                    .change_context(BarentswatchClientError::SendingRequest)?,
            });
        }

        let stream = response.bytes_stream();

        let stream = stream
            .map(|result| {
                result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })
            .into_async_read();

        let compat = tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream);

        Ok(compat)
    }
}
