use crate::*;
use chrono::{DateTime, Datelike, Months, Utc};
use enum_index::EnumIndex;
use enum_index_derive::EnumIndex;
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use matrix_index_error::ValueSnafu;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumCount, EnumIter, EnumString};

use super::compute_sum_area_table;

#[derive(Debug, Clone)]
pub struct HaulWeight {
    pub period: DateRange,
    pub weight: f64,
}

#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(
    Debug,
    Copy,
    Clone,
    Deserialize,
    FromPrimitive,
    Serialize,
    PartialEq,
    EnumIndex,
    strum::Display,
    AsRefStr,
    EnumString,
)]
pub enum ActiveHaulsFilter {
    Date = 1,
    GearGroup = 2,
    SpeciesGroup = 3,
    VesselLength = 4,
}

#[derive(Debug, Copy, Clone, EnumIter, PartialEq, Eq, Display)]
pub enum HaulMatrixes {
    Date,
    GearGroup,
    SpeciesGroup,
    VesselLength,
}

impl PartialEq<HaulMatrixes> for ActiveHaulsFilter {
    fn eq(&self, other: &HaulMatrixes) -> bool {
        match self {
            ActiveHaulsFilter::Date => matches!(other, HaulMatrixes::Date),
            ActiveHaulsFilter::GearGroup => matches!(other, HaulMatrixes::GearGroup),
            ActiveHaulsFilter::SpeciesGroup => matches!(other, HaulMatrixes::SpeciesGroup),
            ActiveHaulsFilter::VesselLength => matches!(other, HaulMatrixes::VesselLength),
        }
    }
}

impl PartialEq<ActiveHaulsFilter> for HaulMatrixXFeature {
    fn eq(&self, other: &ActiveHaulsFilter) -> bool {
        let tmp: HaulMatrixXFeature = (*other).into();
        self.eq(&tmp)
    }
}

fn haul_date_feature_matrix_size() -> usize {
    let diff = chrono::Utc::now() - Months::new(ERS_OLDEST_DATA_MONTHS as u32);
    (diff.month() as i32 + 1 + (diff.year() * 12)) as usize
}

pub fn haul_date_feature_matrix_index(ts: &DateTime<Utc>) -> usize {
    ts.year() as usize * 12 + ts.month0() as usize - ERS_OLDEST_DATA_MONTHS
}

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(
    Default, Debug, Clone, Copy, Deserialize, Serialize, strum::Display, AsRefStr, EnumString,
)]
pub enum HaulsSorting {
    #[serde(alias = "startDate", alias = "StartDate", alias = "START_DATE")]
    #[default]
    StartDate = 1,
    #[serde(alias = "stopDate", alias = "StopDate", alias = "STOP_DATE")]
    StopDate = 2,
    #[serde(alias = "weight", alias = "Weight", alias = "WEIGHT")]
    Weight = 3,
}

#[derive(Default, Debug, Clone)]
pub struct HaulsQuery {
    pub ranges: Vec<Range<DateTime<Utc>>>,
    pub catch_locations: Vec<CatchLocationId>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub vessel_length_groups: Vec<VesselLengthGroup>,
    pub vessel_ids: Vec<FiskeridirVesselId>,
    pub range: OptionalDateTimeRange,
    pub sorting: Option<HaulsSorting>,
    pub ordering: Option<Ordering>,
}

#[derive(Debug, Clone)]
pub struct HaulsMatrixQuery {
    pub months: Vec<u32>,
    pub catch_locations: Vec<CatchLocationId>,
    pub gear_group_ids: Vec<GearGroup>,
    pub species_group_ids: Vec<SpeciesGroup>,
    pub vessel_length_groups: Vec<VesselLengthGroup>,
    pub vessel_ids: Vec<FiskeridirVesselId>,
    pub active_filter: ActiveHaulsFilter,
    pub bycatch_percentage: Option<f64>,
    pub majority_species_group: bool,
}

#[derive(Debug, Clone)]
pub struct HaulMatrixQueryOutput {
    pub sum_living: i64,
    pub x_index: i32,
    pub y_index: i32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HaulMatrixXFeature {
    Date = 0,
    GearGroup = 1,
    SpeciesGroup = 2,
    VesselLength = 3,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum HaulMatrixYFeature {
    Date = 0,
    GearGroup = 1,
    SpeciesGroup = 2,
    VesselLength = 3,
    CatchLocation = 4,
}

impl From<ActiveHaulsFilter> for HaulMatrixes {
    fn from(value: ActiveHaulsFilter) -> Self {
        match value {
            ActiveHaulsFilter::Date => HaulMatrixes::Date,
            ActiveHaulsFilter::GearGroup => HaulMatrixes::GearGroup,
            ActiveHaulsFilter::SpeciesGroup => HaulMatrixes::SpeciesGroup,
            ActiveHaulsFilter::VesselLength => HaulMatrixes::VesselLength,
        }
    }
}

impl From<ActiveHaulsFilter> for HaulMatrixXFeature {
    fn from(value: ActiveHaulsFilter) -> Self {
        match value {
            ActiveHaulsFilter::Date => HaulMatrixXFeature::Date,
            ActiveHaulsFilter::GearGroup => HaulMatrixXFeature::GearGroup,
            ActiveHaulsFilter::SpeciesGroup => HaulMatrixXFeature::SpeciesGroup,
            ActiveHaulsFilter::VesselLength => HaulMatrixXFeature::VesselLength,
        }
    }
}

impl From<ActiveHaulsFilter> for HaulMatrixYFeature {
    fn from(value: ActiveHaulsFilter) -> Self {
        match value {
            ActiveHaulsFilter::Date => HaulMatrixYFeature::Date,
            ActiveHaulsFilter::GearGroup => HaulMatrixYFeature::GearGroup,
            ActiveHaulsFilter::SpeciesGroup => HaulMatrixYFeature::SpeciesGroup,
            ActiveHaulsFilter::VesselLength => HaulMatrixYFeature::VesselLength,
        }
    }
}

impl HaulMatrixes {
    pub fn size(&self) -> usize {
        match self {
            HaulMatrixes::Date => haul_date_feature_matrix_size(),
            HaulMatrixes::GearGroup => GearGroup::COUNT,
            HaulMatrixes::SpeciesGroup => SpeciesGroup::COUNT,
            HaulMatrixes::VesselLength => VesselLengthGroup::COUNT,
        }
    }
}

impl HaulMatrixYFeature {
    pub fn column_name(&self) -> &'static str {
        match self {
            HaulMatrixYFeature::Date => "matrix_month_bucket",
            HaulMatrixYFeature::GearGroup => "gear_group_id",
            HaulMatrixYFeature::SpeciesGroup => "species_group_id",
            HaulMatrixYFeature::VesselLength => "vessel_length_group",
            HaulMatrixYFeature::CatchLocation => "catch_location_matrix_index",
        }
    }
    fn convert_from_val(&self, val: i32) -> Result<usize, MatrixIndexError> {
        match self {
            HaulMatrixYFeature::Date => {
                let converted = val as usize;
                if converted >= ERS_OLDEST_DATA_MONTHS {
                    Ok(converted - ERS_OLDEST_DATA_MONTHS)
                } else {
                    ValueSnafu { val }.fail()
                }
            }
            HaulMatrixYFeature::GearGroup => GearGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            HaulMatrixYFeature::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            HaulMatrixYFeature::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            HaulMatrixYFeature::CatchLocation => Ok(val as usize),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            HaulMatrixYFeature::Date => haul_date_feature_matrix_size(),
            HaulMatrixYFeature::GearGroup => GearGroup::COUNT,
            HaulMatrixYFeature::SpeciesGroup => SpeciesGroup::COUNT,
            HaulMatrixYFeature::VesselLength => VesselLengthGroup::COUNT,
            HaulMatrixYFeature::CatchLocation => NUM_CATCH_LOCATIONS,
        }
    }
}

impl HaulMatrixXFeature {
    fn convert_from_val(&self, val: i32) -> Result<usize, MatrixIndexError> {
        match self {
            HaulMatrixXFeature::Date => {
                let converted = val as usize;
                if converted >= ERS_OLDEST_DATA_MONTHS {
                    Ok(converted - ERS_OLDEST_DATA_MONTHS)
                } else {
                    ValueSnafu { val }.fail()
                }
            }
            HaulMatrixXFeature::GearGroup => GearGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            HaulMatrixXFeature::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            HaulMatrixXFeature::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
        }
    }
    pub fn size(&self) -> usize {
        match self {
            HaulMatrixXFeature::Date => haul_date_feature_matrix_size(),
            HaulMatrixXFeature::GearGroup => GearGroup::COUNT,
            HaulMatrixXFeature::SpeciesGroup => SpeciesGroup::COUNT,
            HaulMatrixXFeature::VesselLength => VesselLengthGroup::COUNT,
        }
    }
    pub fn column_name(&self) -> &'static str {
        match self {
            HaulMatrixXFeature::Date => "matrix_month_bucket",
            HaulMatrixXFeature::GearGroup => "gear_group_id",
            HaulMatrixXFeature::SpeciesGroup => "species_group_id",
            HaulMatrixXFeature::VesselLength => "vessel_length_group",
        }
    }
}

pub fn calculate_haul_sum_area_table(
    x_feature: HaulMatrixXFeature,
    y_feature: HaulMatrixYFeature,
    data: Vec<HaulMatrixQueryOutput>,
) -> Result<Vec<u64>, MatrixIndexError> {
    let height = y_feature.size();
    let width = x_feature.size();

    let mut matrix: Vec<u64> = vec![0; width * height];

    for d in data {
        let x = x_feature.convert_from_val(d.x_index)?;
        let y = y_feature.convert_from_val(d.y_index)?;

        matrix[(y * width) + x] = d.sum_living as u64;
    }

    compute_sum_area_table(&mut matrix, width);

    Ok(matrix)
}
