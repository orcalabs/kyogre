#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod fuel_consumption;
mod sustainability;
mod weight_per_distance;
mod weight_per_hour;

pub use fuel_consumption::*;
pub use sustainability::*;
pub use weight_per_distance::*;
pub use weight_per_hour::*;
