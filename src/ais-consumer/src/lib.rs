#![deny(warnings)]
#![deny(rust_2018_idioms)]

//! Implements a binary that continously consume an input ais-stream from Kysteverket and adds it to our postgres
//! database.

pub mod barentswatch;
pub mod consumer;
pub mod error;
pub mod models;
pub mod settings;
pub mod startup;
