mod ais;
mod ais_vms;
mod catch_location;
mod date_range;
mod delivery_points;
mod ers;
mod fishing_facility;
mod fuel;
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

pub const MIN_EEOI_DISTANCE: f64 = 1_000.;
pub const METERS_TO_NAUTICAL_MILES: f64 = 1. / 1852.;
/// Fuel mass to CO2 mass conversion factor for Diesel/Gas Oil
/// Unit: CO2 (tonn) / Fuel (tonn)
///
/// Source: <https://www.classnk.or.jp/hp/pdf/activities/statutory/eedi/mepc_1-circ_684.pdf>
///         Appendix, section 3
pub const DIESEL_CARBON_FACTOR: f64 = 3.206;

pub use fiskeridir_rs::FiskeridirVesselId;

pub use ais::*;
pub use ais_vms::*;
pub use catch_location::*;
pub use date_range::*;
pub use delivery_points::*;
pub use ers::*;
pub use fishing_facility::*;
pub use fuel::*;
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
