#![deny(warnings)]
#![deny(rust_2018_idioms)]

#[cfg(feature = "client")]
pub mod client;
pub mod error;
mod protobuf;
#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "client")]
pub use client::Client;
#[cfg(feature = "server")]
pub use server::*;
