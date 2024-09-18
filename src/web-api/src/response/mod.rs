use actix_web::{body::BoxBody, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

mod stream;

pub use stream::*;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Response<T> {
    pub body: T,
}

pub enum ResponseOrStream<T> {
    Response(Response<Vec<T>>),
    Stream(StreamResponse<T>),
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

impl<T> Responder for ResponseOrStream<T>
where
    T: Serialize + 'static,
{
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        match self {
            Self::Response(v) => v.respond_to(req),
            Self::Stream(v) => v.respond_to(req),
        }
    }
}

impl<T> From<Response<Vec<T>>> for ResponseOrStream<T> {
    fn from(value: Response<Vec<T>>) -> Self {
        Self::Response(value)
    }
}

impl<T> From<StreamResponse<T>> for ResponseOrStream<T> {
    fn from(value: StreamResponse<T>) -> Self {
        Self::Stream(value)
    }
}
