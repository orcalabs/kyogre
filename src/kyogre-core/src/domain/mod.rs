mod ais;
mod ais_vms;
mod catch_location;
mod date_range;
mod delivery_points;
mod ers;
mod fishing_facility;
mod hauls;
mod landing;
mod ocean_climate;
mod ports;
mod range;
mod species;
mod trips;
mod user;
mod vessels;
mod vms;
mod weather;
mod benchmark;

#[derive(Clone, Debug, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

pub static NUM_CATCH_LOCATIONS: usize = 2113;

pub use ais::*;
pub use ais_vms::*;
pub use catch_location::*;
pub use date_range::*;
pub use delivery_points::*;
pub use ers::*;
pub use fishing_facility::*;
pub use hauls::*;
pub use landing::*;
pub use ocean_climate::*;
pub use ports::*;
pub use range::*;
pub use species::*;
pub use trips::*;
pub use user::*;
pub use vessels::*;
pub use vms::*;
pub use weather::*;
pub use benchmark::*;
