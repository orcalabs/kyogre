#![deny(warnings)]
#![deny(rust_2018_idioms)]

mod catch_value_per_fuel;
mod eeoi;
mod fuel_consumption;
mod weight_per_distance;
mod weight_per_fuel;
mod weight_per_hour;

pub use catch_value_per_fuel::*;
pub use eeoi::*;
pub use fuel_consumption::*;
pub use weight_per_distance::*;
pub use weight_per_fuel::*;
pub use weight_per_hour::*;
