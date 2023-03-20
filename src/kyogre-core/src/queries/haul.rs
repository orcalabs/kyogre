use fiskeridir_rs::GearGroup;

use crate::{CatchLocationId, DateRange, Range};

pub struct HaulsQuery {
    pub ranges: Option<Vec<DateRange>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<u32>>,
    pub vessel_length_ranges: Option<Vec<Range<f64>>>,
}
