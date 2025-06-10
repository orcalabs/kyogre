use crate::error::{Result, error::FailedRequestSnafu};
use futures::{StreamExt, TryStreamExt};
use kyogre_core::{BearerToken, OauthConfig};
use reqwest::{Client, Url};
use serde::Serialize;
use tokio::io::AsyncRead;

pub struct BarentswatchAisClient {
    oauth_config: OauthConfig,
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
    pub fn new(oauth_config: OauthConfig, api_address: Url) -> BarentswatchAisClient {
        BarentswatchAisClient {
            oauth_config,
            api_address,
            client: Client::new(),
        }
    }

    /// Returns the ais source as stream which will continuously receive data from the source.
    pub async fn streamer(&self) -> Result<impl AsyncRead> {
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
                format!(
                    "bearer {}",
                    BearerToken::acquire(&self.oauth_config).await?.as_ref()
                ),
            )
            .header("Content-type", "application/json")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            return FailedRequestSnafu {
                url: self.api_address.clone(),
                status,
                body: response.text().await?,
            }
            .fail();
        }

        let stream = response.bytes_stream();

        let stream = stream
            .map(|result| result.map_err(|e| std::io::Error::other(format!("{e:?}"))))
            .into_async_read();

        let compat = tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream);

        Ok(compat)
    }
}
