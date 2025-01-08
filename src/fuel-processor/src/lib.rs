#![deny(warnings)]
#![deny(rust_2018_idioms)]

pub mod error;
pub mod fuel_estimation;
pub mod live_fuel;
pub mod settings;
pub mod startup;
pub mod unrealistic_speed;

pub use error::*;
pub use fuel_estimation::*;
pub use live_fuel::*;
pub use settings::*;
pub use startup::*;
pub use unrealistic_speed::*;
