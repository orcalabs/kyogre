use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    FiskeridirVesselId, Gear, GearGroup, SpeciesGroup, SpeciesMainGroup, VesselLengthGroup,
    WhaleGender,
};
use serde::{Deserialize, Serialize};

use crate::{CatchLocationId, HaulOceanClimate, HaulWeather, ProcessingStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct HaulId(i64);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[remain::sorted]
pub struct Haul {
    pub cache_version: i64,
    pub catch_location_start: Option<CatchLocationId>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub catches: Vec<HaulCatch>,
    pub duration: i32,
    pub ers_activity_id: String,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub gear_group_id: GearGroup,
    pub gear_id: Gear,
    pub haul_distance: Option<i32>,
    pub haul_id: HaulId,
    pub ocean_climate: HaulOceanClimate,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub stop_timestamp: DateTime<Utc>,
    pub total_living_weight: i64,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: f64,
    pub vessel_length_group: VesselLengthGroup,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub weather: HaulWeather,
    pub whale_catches: Vec<WhaleCatch>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[remain::sorted]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fao_id: String,
    pub species_fiskeridir_id: i32,
    pub species_group_id: SpeciesGroup,
    pub species_main_group_id: Option<SpeciesMainGroup>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[remain::sorted]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    pub gender_id: Option<WhaleGender>,
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HaulsMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HaulWeatherStatus {
    Unprocessed = 1,
    Attempted = 2,
    Successful = 3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaulMessage {
    pub haul_id: HaulId,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaulDistributionOutput {
    pub haul_id: HaulId,
    pub catch_location: CatchLocationId,
    pub factor: f64,
    pub status: ProcessingStatus,
}

impl HaulId {
    pub fn into_inner(self) -> i64 {
        self.0
    }
    pub fn test_new(value: i64) -> Self {
        Self(value)
    }
}

impl From<HaulId> for i64 {
    fn from(value: HaulId) -> Self {
        value.0
    }
}

impl Display for HaulId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
