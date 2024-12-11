use chrono::{DateTime, Utc};
use fiskeridir_rs::SpeciesGroup;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;

use crate::*;

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrgBenchmarks {
    /// Unit is in seconds
    pub fishing_time: u64,
    /// Unit is in meters
    pub trip_distance: f64,
    /// Unit is in seconds
    pub trip_time: u64,
    /// Unit is in KG
    pub landing_total_living_weight: f64,
    pub vessels: Vec<OrgBenchmarkEntry>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrgBenchmarkEntry {
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    /// Unit is in seconds
    pub fishing_time: u64,
    /// Unit is in meters
    pub trip_distance: f64,
    /// Unit is in seconds
    pub trip_time: u64,
    /// Unit is in KG
    pub landing_total_living_weight: f64,
    pub species: Vec<OrgBenchmarkSpecies>,
}

#[serde_as]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct OrgBenchmarkSpecies {
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
    /// Unit is in KG
    pub landing_total_living_weight: f64,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VesselBenchmarks {
    /// Time unit is in minutes
    pub fishing_time: Option<Benchmark>,
    /// Distance unit is in meters
    pub fishing_distance: Option<Benchmark>,
    /// Time unit is in minutes
    pub trip_time: Option<Benchmark>,
    pub landings: Option<Benchmark>,
    pub ers_dca: Option<Benchmark>,
    pub cumulative_landings: Vec<CumulativeLandings>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Benchmark {
    pub average: f64,
    pub average_followers: f64,
    pub recent_trips: Vec<BenchmarkEntry>,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkEntry {
    #[cfg_attr(feature = "utoipa", schema(value_type = i64))]
    pub fiskeridir_vessel_id: FiskeridirVesselId,
    pub trip_start: DateTime<Utc>,
    pub value: f64,
}

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CumulativeLandings {
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub month: chrono::Month,
    pub species_fiskeridir_id: u32,
    pub weight: f64,
    pub cumulative_weight: f64,
}

impl OrgBenchmarkEntry {
    pub fn is_empty(&self) -> bool {
        self.fishing_time.is_zero()
            && self.trip_distance.is_zero()
            && self.trip_time.is_zero()
            && self.landing_total_living_weight.is_zero()
    }
}

impl PartialEq<(&TripDetailed, f64)> for BenchmarkEntry {
    fn eq(&self, other: &(&TripDetailed, f64)) -> bool {
        let Self {
            fiskeridir_vessel_id,
            trip_start,
            value,
        } = self;

        *value as i64 == other.1 as i64
            && *fiskeridir_vessel_id == other.0.fiskeridir_vessel_id
            && *trip_start == other.0.period.start()
    }
}

impl PartialEq<BenchmarkEntry> for (&TripDetailed, f64) {
    fn eq(&self, other: &BenchmarkEntry) -> bool {
        other.eq(self)
    }
}
