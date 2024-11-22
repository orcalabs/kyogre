use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::*;

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
