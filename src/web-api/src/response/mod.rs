use std::any::TypeId;

use actix_web::{HttpRequest, HttpResponse, Responder, body::BoxBody};
use oasgen::{OaSchema, ObjectType, RefOr, Schema, SchemaData, SchemaKind, Type};
use serde::{Deserialize, Serialize};

mod stream;

pub use stream::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T> {
    pub body: T,
}

pub enum ResponseOrStream<T> {
    Response(Response<Vec<T>>),
    Stream(StreamResponse<T>),
}

impl<T: OaSchema + 'static> OaSchema for Response<T> {
    fn schema_ref() -> oasgen::ReferenceOr<Schema> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            RefOr::Item(Schema {
                data: SchemaData::default(),
                kind: SchemaKind::Type(Type::Object(ObjectType::default())),
            })
        } else {
            T::schema_ref()
        }
    }

    fn schema() -> Schema {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            Schema {
                data: SchemaData::default(),
                kind: SchemaKind::Type(Type::Object(ObjectType::default())),
            }
        } else {
            T::schema()
        }
    }
}

impl<T: OaSchema + 'static> OaSchema for ResponseOrStream<T> {
    fn schema_ref() -> oasgen::ReferenceOr<Schema> {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            RefOr::Item(Schema {
                data: SchemaData::default(),
                kind: SchemaKind::Type(Type::Object(ObjectType::default())),
            })
        } else {
            Vec::<T>::schema_ref()
        }
    }

    fn schema() -> Schema {
        if TypeId::of::<T>() == TypeId::of::<()>() {
            Schema {
                data: SchemaData::default(),
                kind: SchemaKind::Type(Type::Object(ObjectType::default())),
            }
        } else {
            Vec::<T>::schema()
        }
    }
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
