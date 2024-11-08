use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

use crate::{
    FishingFacilities, FishingFacilityToolType, FiskeridirVesselId, Mmsi, Ordering, Pagination,
    Range,
};

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(
    Default, Debug, Clone, Copy, Deserialize, Serialize, strum::Display, AsRefStr, EnumString,
)]
pub enum FishingFacilitiesSorting {
    #[serde(alias = "setup", alias = "Setup", alias = "SETUP")]
    #[default]
    Setup = 1,
    #[serde(alias = "removed", alias = "Removed", alias = "REMOVED")]
    Removed = 2,
    #[serde(alias = "last_changed", alias = "LastChanged", alias = "LAST_CHANGED")]
    LastChanged = 3,
}

#[derive(Debug, Clone)]
pub struct FishingFacilitiesQuery {
    pub mmsis: Vec<Mmsi>,
    pub fiskeridir_vessel_ids: Vec<FiskeridirVesselId>,
    pub tool_types: Vec<FishingFacilityToolType>,
    pub active: Option<bool>,
    pub setup_ranges: Vec<Range<DateTime<Utc>>>,
    pub removed_ranges: Vec<Range<DateTime<Utc>>>,
    pub pagination: Pagination<FishingFacilities>,
    pub ordering: Option<Ordering>,
    pub sorting: Option<FishingFacilitiesSorting>,
}
