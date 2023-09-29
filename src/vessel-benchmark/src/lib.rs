#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod weight_per_hour;
mod weight_per_distance;
mod weight_per_hour_interval;
mod weight_per_distance_interval;
mod total_weight;

pub use weight_per_hour::*;
pub use weight_per_distance::*;
pub use weight_per_hour_interval::*;
pub use weight_per_distance_interval::*;
pub use total_weight::*;

