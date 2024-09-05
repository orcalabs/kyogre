mod client;
mod error;
mod request;
mod response;

pub use reqwest::{header::HeaderMap, StatusCode};

pub use client::{HttpClient, HttpClientBuilder};
pub use error::{Error, Result};
pub use request::RequestBuilder;
pub use response::Response;
