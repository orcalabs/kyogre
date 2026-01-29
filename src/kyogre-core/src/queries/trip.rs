use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

use crate::{FiskeridirVesselId, OptionalDateTimeRange, Ordering, Pagination, TripId, Trips};

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(
    Default, Debug, Clone, Copy, Deserialize, Serialize, strum::Display, AsRefStr, EnumString,
)]
pub enum TripSorting {
    #[serde(alias = "stopDate", alias = "StopDate", alias = "STOP_DATE")]
    #[default]
    StopDate = 1,
    #[serde(alias = "weight", alias = "Weight", alias = "WEIGHT")]
    Weight = 2,
}

#[repr(i32)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum HasTrack {
    NoTrack = 1,
    TrackUnder15 = 2,
    TrackOver15 = 3,
}

#[derive(Debug, Clone, Default)]
pub struct TripsQuery {
    pub pagination: Pagination<Trips>,
    pub ordering: Ordering,
    pub sorting: TripSorting,
    pub delivery_points: Option<Vec<String>>,
    pub range: OptionalDateTimeRange,
    pub min_weight: Option<f64>,
    pub max_weight: Option<f64>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub trip_ids: Option<Vec<TripId>>,
}
