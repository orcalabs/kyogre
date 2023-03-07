use crate::{CatchLocationId, DateRange};

pub struct HaulsQuery {
    pub ranges: Option<Vec<DateRange>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
}
