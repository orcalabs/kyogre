mod client;
mod error;
mod request;
mod response;

pub use reqwest::{StatusCode, header::HeaderMap};

pub use client::{HttpClient, HttpClientBuilder};
pub use error::{Error, Result};
pub use request::RequestBuilder;
pub use response::Response;
