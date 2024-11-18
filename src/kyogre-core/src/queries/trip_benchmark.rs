use chrono::{DateTime, Utc};
use fiskeridir_rs::{CallSign, GearGroup, VesselLengthGroup};

use super::Ordering;

#[derive(Debug, Clone)]
pub struct TripBenchmarksQuery {
    pub call_sign: CallSign,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub ordering: Ordering,
}

#[derive(Debug, Clone)]
pub struct AverageTripBenchmarksQuery {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub gear_groups: Vec<GearGroup>,
    pub length_group: Option<VesselLengthGroup>,
}

#[derive(Debug, Clone)]
pub struct EeoiQuery {
    pub call_sign: CallSign,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AverageEeoiQuery {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub gear_groups: Vec<GearGroup>,
    pub length_group: Option<VesselLengthGroup>,
}
