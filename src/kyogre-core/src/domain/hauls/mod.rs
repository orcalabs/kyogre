use std::fmt::{self, Display};

use chrono::{DateTime, Duration, Utc};
use fiskeridir_rs::{
    CallSign, FiskeridirVesselId, Gear, GearGroup, SpeciesGroup, SpeciesMainGroup,
    VesselLengthGroup, WhaleGender,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "oasgen")]
use oasgen::OaSchema;

use crate::{
    ActiveHaulsFilter, CatchLocationId, HaulMatrixXFeature, HaulMatrixYFeature, ProcessingStatus,
};

use super::TripId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct HaulId(i64);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Haul {
    #[serde(alias = "haul_id")]
    pub id: HaulId,
    pub trip_id: Option<TripId>,
    pub cache_version: i64,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_id: GearGroup,
    pub gear_id: Gear,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub haul_distance: Option<i32>,
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    pub vessel_length_group: VesselLengthGroup,
    pub catches: Vec<HaulCatch>,
    pub vessel_name: Option<String>,
    pub call_sign: CallSign,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fao_id: String,
    pub species_fiskeridir_id: i32,
    pub species_group_id: SpeciesGroup,
    pub species_main_group_id: Option<SpeciesMainGroup>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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

impl Haul {
    pub fn duration(&self) -> Duration {
        self.stop_timestamp - self.start_timestamp
    }
    pub fn total_living_weight(&self) -> i32 {
        self.catches.iter().map(|c| c.living_weight).sum()
    }
}

impl HaulsMatrix {
    pub fn is_empty(&self) -> bool {
        let Self {
            dates,
            length_group,
            gear_group,
            species_group,
        } = self;

        dates.iter().all(|v| *v == 0)
            && length_group.iter().all(|v| *v == 0)
            && gear_group.iter().all(|v| *v == 0)
            && species_group.iter().all(|v| *v == 0)
    }

    pub fn empty(active_filter: ActiveHaulsFilter) -> Self {
        let x_feature: HaulMatrixXFeature = active_filter.into();
        let dates_size = if x_feature == HaulMatrixXFeature::Date {
            HaulMatrixYFeature::Date.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::Date.size() * x_feature.size()
        };

        let length_group_size = if x_feature == HaulMatrixXFeature::VesselLength {
            HaulMatrixYFeature::VesselLength.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::VesselLength.size() * x_feature.size()
        };

        let gear_group_size = if x_feature == HaulMatrixXFeature::GearGroup {
            HaulMatrixYFeature::GearGroup.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::GearGroup.size() * x_feature.size()
        };

        let species_group_size = if x_feature == HaulMatrixXFeature::SpeciesGroup {
            HaulMatrixYFeature::SpeciesGroup.size() * HaulMatrixYFeature::CatchLocation.size()
        } else {
            HaulMatrixYFeature::SpeciesGroup.size() * x_feature.size()
        };

        Self {
            dates: vec![0; dates_size],
            length_group: vec![0; length_group_size],
            gear_group: vec![0; gear_group_size],
            species_group: vec![0; species_group_size],
        }
    }
}

#[cfg(feature = "test")]
mod test {
    use super::*;

    impl HaulId {
        pub fn test_new(value: i64) -> Self {
            Self(value)
        }
    }
}
