use actix_web::{body::BoxBody, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
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
