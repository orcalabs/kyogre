#![deny(warnings)]
#![deny(rust_2018_idioms)]

use kyogre_core::*;

mod error;
mod haul_distributor;
mod trip_assembler;
mod trip_distancer;

pub mod settings;
pub mod startup;

pub use haul_distributor::*;
pub use settings::*;
pub use startup::*;
pub use trip_assembler::*;
pub use trip_distancer::*;

#[derive(Default)]
pub struct AisVms {}
