use crate::error::BarentswatchClientError;
use error_stack::{bail, IntoReport, Result, ResultExt};

use futures::{StreamExt, TryStreamExt};
use hyper::{Body, Client, Request, StatusCode, Uri};
use hyper_alpn::AlpnConnector;
use kyogre_core::BearerToken;
use serde::Serialize;
use tokio::io::AsyncRead;

pub struct BarentswatchAisClient {
    bearer_token: BearerToken,
    api_address: Uri,
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
    pub fn new(bearer_token: BearerToken, api_address: Uri) -> BarentswatchAisClient {
        BarentswatchAisClient {
            bearer_token,
            api_address,
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

        let body = serde_json::to_string(&args)
            .into_report()
            .change_context(BarentswatchClientError::RequestCreation)?;

        let request = Request::post(&self.api_address)
            .header(
                "Authorization",
                format!("bearer {}", self.bearer_token.as_ref()),
            )
            .header("Content-type", "application/json")
            .body(Body::from(body))
            .into_report()
            .change_context(BarentswatchClientError::RequestCreation)?;

        let alpn = AlpnConnector::new();
        let client = Client::builder().http2_only(true).build(alpn);

        let response = client
            .request(request)
            .await
            .into_report()
            .change_context(BarentswatchClientError::SendingRequest)?;

        let status = response.status();
        if status != StatusCode::OK {
            let body = hyper::body::to_bytes(response.into_body())
                .await
                .into_report()
                .change_context(BarentswatchClientError::Body)?;
            bail!(BarentswatchClientError::Server {
                response_code: status.as_u16(),
                body: std::str::from_utf8(&body).unwrap().to_string(),
            });
        }

        let stream = response
            .into_body()
            .map(|result| {
                result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })
            .into_async_read();

        let compat = tokio_util::compat::FuturesAsyncReadCompatExt::compat(stream);
        Ok(compat)
    }
}
