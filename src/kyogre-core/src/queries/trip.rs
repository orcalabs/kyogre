use chrono::{DateTime, Utc};
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use serde::Deserialize;
use strum::{AsRefStr, EnumString};

use crate::{FiskeridirVesselId, Ordering, Pagination, Trips};

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Default, Debug, Clone, Copy, Deserialize, strum::Display, AsRefStr, EnumString)]
pub enum TripSorting {
    #[serde(alias = "stopDate", alias = "StopDate", alias = "STOP_DATE")]
    #[default]
    StopDate = 1,
    #[serde(alias = "weight", alias = "Weight", alias = "WEIGHT")]
    Weight = 2,
}

#[derive(Debug, Clone, Default)]
pub struct TripsQuery {
    pub pagination: Pagination<Trips>,
    pub ordering: Ordering,
    pub sorting: TripSorting,
    pub delivery_points: Option<Vec<String>>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub min_weight: Option<f64>,
    pub max_weight: Option<f64>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub fiskeridir_vessel_ids: Option<Vec<FiskeridirVesselId>>,
}
