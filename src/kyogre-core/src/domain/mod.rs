mod ais;
mod date_range;
mod delivery_points;
mod ers;
mod ports;
mod trips;
mod vessels;

#[derive(Clone, Debug, PartialEq)]
pub struct Coordinates {
    pub latitude: f64,
    pub longitude: f64,
}

pub use ais::*;
pub use date_range::*;
pub use delivery_points::*;
pub use ers::*;
pub use ports::*;
pub use trips::*;
pub use vessels::*;
