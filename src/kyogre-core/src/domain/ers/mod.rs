mod arrival;
mod departure;

pub use arrival::*;
pub use departure::*;

pub static ERS_OLDEST_DATA_MONTHS: usize = 2010 * 12;
