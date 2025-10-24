use crate::{Ordering, Pagination, VesselEventType, VesselEvents};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct VesselEventQuery {
    pub start_timestamp: Option<DateTime<Utc>>,
    pub end_timestamp: Option<DateTime<Utc>>,
    pub vessel_event_type: Option<VesselEventType>,
    pub ordering: Option<Ordering>,
    pub pagination: Pagination<VesselEvents>,
}
