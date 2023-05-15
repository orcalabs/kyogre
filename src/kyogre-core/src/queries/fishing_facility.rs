use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;

use crate::{FishingFacilityToolType, Mmsi, Range};

#[derive(Debug, Clone)]
pub struct FishingFacilitiesQuery {
    pub mmsis: Option<Vec<Mmsi>>,
    pub call_signs: Option<Vec<CallSign>>,
    pub tool_types: Option<Vec<FishingFacilityToolType>>,
    pub active: Option<bool>,
    pub setup_ranges: Option<Vec<Range<DateTime<Utc>>>>,
    pub removed_ranges: Option<Vec<Range<DateTime<Utc>>>>,
}
