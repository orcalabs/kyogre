#![deny(warnings)]
#![deny(rust_2018_idioms)]

use chrono::{DateTime, Duration, Utc};
use error_stack::Result;

mod error;
mod ers;
mod landings;
mod precision;

pub use error::*;
pub use ers::*;
pub use landings::*;
pub use precision::*;

// TODO: make this a const when rust supports it
pub fn ers_last_trip_landing_coverage_end(last_trip_end: &DateTime<Utc>) -> DateTime<Utc> {
    *last_trip_end + Duration::days(3)
}
