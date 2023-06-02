use crate::*;
use chrono::{DateTime, Datelike, Months, Utc};
use enum_index::EnumIndex;
use enum_index_derive::EnumIndex;
use error_stack::{IntoReport, Result};
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumCount, EnumIter};

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq, EnumIndex)]
#[serde(rename_all = "camelCase")]
pub enum ActiveHaulsFilter {
    Date,
    GearGroup,
    SpeciesGroup,
    VesselLength,
}

#[derive(Debug, Copy, Clone, EnumIter, PartialEq, Eq, Display)]
pub enum HaulMatrixes {
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

pub struct MatrixQueryOutput {
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
            HaulMatrixes::Date => date_feature_matrix_size(),
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
            HaulMatrixYFeature::CatchLocation => "catch_location_start_matrix_index",
        }
    }
    fn convert_from_val(&self, val: i32) -> Result<usize, HaulMatrixIndexError> {
        match self {
            HaulMatrixYFeature::Date => {
                let converted = val as usize;
                if converted >= ERS_OLDEST_DATA_MONTHS {
                    Ok(converted - ERS_OLDEST_DATA_MONTHS)
                } else {
                    Err(HaulMatrixIndexError::Date(val))
                }
            }
            HaulMatrixYFeature::GearGroup => GearGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::GearGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixYFeature::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::SpeciesGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixYFeature::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::VesselLength(val))
                .map(|v| v.enum_index()),
            HaulMatrixYFeature::CatchLocation => Ok(val as usize),
        }
        .into_report()
    }

    fn size(&self) -> usize {
        match self {
            HaulMatrixYFeature::Date => date_feature_matrix_size(),
            HaulMatrixYFeature::GearGroup => GearGroup::COUNT,
            HaulMatrixYFeature::SpeciesGroup => SpeciesGroup::COUNT,
            HaulMatrixYFeature::VesselLength => VesselLengthGroup::COUNT,
            HaulMatrixYFeature::CatchLocation => NUM_CATCH_LOCATIONS,
        }
    }
}

impl HaulMatrixXFeature {
    fn convert_from_val(&self, val: i32) -> Result<usize, HaulMatrixIndexError> {
        match self {
            HaulMatrixXFeature::Date => {
                let converted = val as usize;
                if converted >= ERS_OLDEST_DATA_MONTHS {
                    Ok(converted - ERS_OLDEST_DATA_MONTHS)
                } else {
                    Err(HaulMatrixIndexError::Date(val))
                }
            }
            HaulMatrixXFeature::GearGroup => GearGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::GearGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixXFeature::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::SpeciesGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixXFeature::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::VesselLength(val))
                .map(|v| v.enum_index()),
        }
        .into_report()
    }
    fn size(&self) -> usize {
        match self {
            HaulMatrixXFeature::Date => date_feature_matrix_size(),
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

pub fn calculate_sum_area_table(
    x_feature: HaulMatrixXFeature,
    y_feature: HaulMatrixYFeature,
    data: Vec<MatrixQueryOutput>,
) -> Result<Vec<u64>, HaulMatrixIndexError> {
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

fn compute_sum_area_table(input: &mut [u64], width: usize) {
    let mut i = 0;

    while i < input.len() {
        let mut sum = input[i];

        let y = i / width;
        let x = i % width;

        if y > 0 {
            let idx = (width * (y - 1)) + x;
            sum += input[idx];
        }
        if x > 0 {
            let idx = (width * y) + (x - 1);
            sum += input[idx];
        }
        if x > 0 && y > 0 {
            let idx = (width * (y - 1)) + (x - 1);
            sum -= input[idx];
        }
        input[i] = sum;

        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_area_table() {
        let mut input = vec![1, 2, 3, 4, 6, 5, 3, 8, 1, 2, 4, 6, 7, 5, 5, 2, 4, 8, 9, 4];
        compute_sum_area_table(&mut input, 5);
        assert_eq!(
            vec![1, 3, 6, 10, 16, 6, 11, 22, 27, 35, 10, 21, 39, 49, 62, 12, 27, 53, 72, 89],
            input
        );
    }
}
