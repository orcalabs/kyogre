use actix_web::{body::BoxBody, web::Bytes, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::{event, Level};
use utoipa::ToSchema;

use crate::error::ApiError;

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

pub fn to_bytes<T: Serialize>(value: &T) -> Result<Bytes, ApiError> {
    let json = serde_json::to_vec(value).map_err(|e| {
        event!(Level::ERROR, "failed to serialize value: {:?}", e);
        ApiError::InternalServerError
    })?;

    Ok(Bytes::from(json))
}

#[macro_export]
macro_rules! to_streaming_response {
    ($stream:expr) => {
        use actix_web::{http::header::ContentType, web::Bytes, HttpResponse};
        use async_stream::{__private::AsyncStream, try_stream};
        use futures::StreamExt;

        use $crate::error::ApiError;
        use $crate::response::to_bytes;

        let stream: AsyncStream<Result<Bytes, ApiError>, _> = try_stream! {
            let mut stream = $stream;

            yield Bytes::from_static(b"[");

            if let Some(first) = stream.next().await {
                yield to_bytes(&first?)?;
            }

            while let Some(item) = stream.next().await {
                yield Bytes::from_static(b",");
                yield to_bytes(&item?)?;
            }

            yield Bytes::from_static(b"]");
        };

        Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .streaming(Box::pin(stream)))
    };
}
