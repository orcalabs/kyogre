mod arrival;
mod departure;

pub use arrival::*;
use chrono::{DateTime, Utc};
pub use departure::*;

pub static ERS_OLDEST_DATA_MONTHS: usize = 2010 * 12;

#[derive(Debug, Clone)]
pub struct ErsDcaId {
    pub message_id: i64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
}
