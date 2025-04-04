use chrono::Duration;
use serde::{Deserialize, Serialize};

mod arrival;
mod departure;
mod tra;

pub use arrival::*;
pub use departure::*;
pub use tra::*;

pub static ERS_OLDEST_DATA_MONTHS: usize = 2010 * 12;
pub static ERS_LANDING_COVERAGE_OFFSET: Duration = Duration::hours(6);

#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum ErsQuantumType {
    /// Catch transferred
    KG,
    /// Catch onboard
    OB,
}
