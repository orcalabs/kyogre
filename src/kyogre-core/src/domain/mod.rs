mod ais;
mod catch_location;
mod date_range;
mod delivery_points;
mod ers;
mod haul;
mod ports;
mod species;
mod trips;
mod vessels;

#[derive(Clone, Debug, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

pub use ais::*;
pub use catch_location::*;
pub use date_range::*;
pub use delivery_points::*;
pub use ers::*;
pub use haul::*;
pub use ports::*;
pub use species::*;
pub use trips::*;
pub use vessels::*;
