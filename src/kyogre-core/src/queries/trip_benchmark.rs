use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;

use super::Ordering;

#[derive(Debug, Clone)]
pub struct TripBenchmarksQuery {
    pub call_sign: CallSign,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub ordering: Ordering,
}
