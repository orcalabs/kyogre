mod ais;
mod ais_vms;
mod catch_location;
mod date_range;
mod delivery_points;
mod ers;
mod fishing_facility;
mod hauls;
mod landing;
mod min_max_both;
mod ml_models;
mod ocean_climate;
mod ports;
mod range;
mod species;
mod trips;
mod user;
mod vessels;
mod vms;
mod weather;

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessingStatus {
    Unprocessed = 1,
    Attempted = 2,
    Successful = 3,
}

impl From<ProcessingStatus> for i32 {
    fn from(value: ProcessingStatus) -> Self {
        value as i32
    }
}

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
pub use min_max_both::*;
pub use ml_models::*;
pub use ocean_climate::*;
pub use ports::*;
pub use range::*;
pub use species::*;
pub use trips::*;
pub use user::*;
pub use vessels::*;
pub use vms::*;
pub use weather::*;
