use crate::{CatchLocationId, FiskeridirVesselId, Range, ERS_OLDEST_DATA_MONTHS};
use chrono::{DateTime, Datelike, Months, Utc};
use enum_index_derive::EnumIndex;
use fiskeridir_rs::{GearGroup, VesselLengthGroup};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq, EnumIndex)]
#[serde(rename_all = "camelCase")]
pub enum ActiveHaulsFilter {
    Date,
    GearGroup,
    SpeciesGroup,
    VesselLength,
}

impl ActiveHaulsFilter {
    pub fn name(&self) -> &'static str {
        match self {
            ActiveHaulsFilter::Date => "date",
            ActiveHaulsFilter::GearGroup => "gearGroup",
            ActiveHaulsFilter::SpeciesGroup => "speciesGroup",
            ActiveHaulsFilter::VesselLength => "vesselLength",
        }
    }
}

pub fn date_feature_matrix_size() -> usize {
    let diff = chrono::Utc::now() - Months::new(ERS_OLDEST_DATA_MONTHS as u32);
    (diff.month() as i32 + (diff.year() * 12)) as usize
}

pub fn date_feature_matrix_index(ts: &DateTime<Utc>) -> usize {
    ts.year() as usize * 12 + ts.month0() as usize - ERS_OLDEST_DATA_MONTHS
}

#[derive(Debug, Clone)]
pub struct HaulsQuery {
    pub ranges: Option<Vec<Range<DateTime<Utc>>>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<u32>>,
    pub vessel_length_ranges: Option<Vec<Range<f64>>>,
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
}

#[derive(Debug, Clone)]
pub struct HaulsMatrixQuery {
    pub months: Option<Vec<u32>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<u32>>,
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub active_filter: ActiveHaulsFilter,
}
