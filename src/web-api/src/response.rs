use std::sync::LazyLock;

use crate::error::Result;
use actix_web::{body::BoxBody, web::Bytes, HttpRequest, HttpResponse, Responder};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

pub static AIS_DETAILS_INTERVAL: LazyLock<Duration> = LazyLock::new(|| Duration::minutes(30));
pub static MISSING_DATA_DURATION: LazyLock<Duration> = LazyLock::new(|| Duration::minutes(70));

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Response<T> {
    pub body: T,
}

impl<T> Response<T> {
    pub fn new(body: T) -> Self {
        Response { body }
    }
}

impl<T> Responder for Response<T>
where
    T: Serialize,
{
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(self.body)
    }
}

impl<T> From<Response<T>> for HttpResponse
where
    T: Serialize,
{
    fn from(v: Response<T>) -> Self {
        HttpResponse::Ok().json(v.body)
    }
}

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Bytes> {
    Ok(Bytes::from(serde_json::to_vec(value)?))
}

pub fn produce_streaming_item<T: Serialize>(
    value: std::result::Result<T, kyogre_core::Error>,
) -> Option<Bytes> {
    let item = value
        .inspect_err(|e| {
            error!(error = true, "failed to retrieve streaming item: {e:?}");
        })
        .ok()?;
    to_bytes(&item)
        .inspect_err(|e| {
            error!(error = true, "failed to serialize streaming item: {e:?}");
        })
        .ok()
}

pub fn produce_ais_streaming_item<T: Serialize>(value: &T) -> Option<Bytes> {
    to_bytes(value)
        .inspect_err(|e| {
            error!(error = true, "failed to serialize streaming item: {e:?}");
        })
        .ok()
}

#[macro_export]
macro_rules! to_streaming_response {
    ($stream:expr) => {
        use actix_web::{http::header::ContentType, web::Bytes, HttpResponse};
        use async_stream::{__private::AsyncStream, try_stream};
        use futures::StreamExt;

        use $crate::error::Result;
        use $crate::response::produce_streaming_item;

        let stream: AsyncStream<Result<Bytes>, _> = try_stream! {
            let mut stream = $stream;

            yield Bytes::from_static(b"[");

            if let Some(first) = stream.next().await {
                if let Some(item) = produce_streaming_item(first) {
                    yield item;
                }
            }

            while let Some(item) = stream.next().await {
                if let Some(item) = produce_streaming_item(item) {
                    yield Bytes::from_static(b",");
                    yield item;
                }
            }

            yield Bytes::from_static(b"]");
        };

        Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .streaming(Box::pin(stream)))
    };
}

#[macro_export]
macro_rules! ais_to_streaming_response {
    ($stream:expr) => {
        use actix_web::{http::header::ContentType, web::Bytes, HttpResponse};
        use async_stream::{__private::AsyncStream, try_stream};
        use futures::{StreamExt, TryStreamExt};
        use tracing::error;

        use $crate::error::Result;
        use $crate::response::{ AIS_DETAILS_INTERVAL, MISSING_DATA_DURATION, produce_ais_streaming_item};

        let stream: AsyncStream<Result<Bytes>, _> = try_stream! {
            let mut stream = $stream.enumerate();

            yield web::Bytes::from_static(b"[");

            let mut missing_flag = false;

            if let Some((_, first)) = stream.next().await {
                let mut pos = first.inspect_err(|e| {
                    error!(error = true, "failed to retrieve streaming item: {e:?}");
                })?;

                let mut prev_details = pos.timestamp;

                while let Some((i, next)) = stream.next().await {
                    let next = match next {
                        Err(e) => {
                            error!(error = true, "failed to retrieve streaming item: {e:?}");
                            continue;
                        }
                        Ok(v) => v,
                    };

                    if next.timestamp - pos.timestamp >= *MISSING_DATA_DURATION {
                        if let Some(ref mut det) = pos.det {
                            det.missing_data = true;
                            missing_flag = true;
                        }
                    } else {
                        if !missing_flag && i != 1 && pos.timestamp - prev_details < *AIS_DETAILS_INTERVAL {
                            pos.det = None;
                        }
                        missing_flag = false;
                    }

                    if pos.det.is_some() {
                        prev_details = pos.timestamp;
                    }

                    if let Some(bytes) = produce_ais_streaming_item(&pos) {
                        yield bytes;
                        yield web::Bytes::from_static(b",");
                        pos = next;
                    }
                }

                if let Some(bytes) = produce_ais_streaming_item(&pos) {
                    yield bytes;
                }
            }

            yield web::Bytes::from_static(b"]");
        };

        Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .streaming(Box::pin(stream)))
    };
}
