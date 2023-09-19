#![deny(warnings)]
#![deny(rust_2018_idioms)]

use kyogre_core::*;

mod error;
mod trip_assembler;
mod trip_distancer;

pub mod settings;
pub mod startup;

pub use settings::*;
pub use startup::*;
pub use trip_assembler::*;
pub use trip_distancer::*;
