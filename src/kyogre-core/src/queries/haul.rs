use crate::*;
use chrono::{DateTime, Datelike, Months, Utc};
use enum_index::EnumIndex;
use enum_index_derive::EnumIndex;
use error_stack::{IntoReport, Result};
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumCount;

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(Debug, Copy, Clone, Deserialize, Serialize, PartialEq, EnumIndex)]
#[serde(rename_all = "camelCase")]
pub enum ActiveHaulsFilter {
    Date,
    GearGroup,
    SpeciesGroup,
    VesselLength,
    CatchLocation,
}

impl ActiveHaulsFilter {
    pub fn name(&self) -> &'static str {
        match self {
            ActiveHaulsFilter::Date => "date",
            ActiveHaulsFilter::GearGroup => "gearGroup",
            ActiveHaulsFilter::SpeciesGroup => "speciesGroup",
            ActiveHaulsFilter::VesselLength => "vesselLength",
            ActiveHaulsFilter::CatchLocation => "catchLocation",
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

pub struct MatrixQueryOutput {
    pub sum_living: i64,
    pub x_index: i32,
    pub y_index: i32,
}

#[derive(Debug, Copy, Clone)]
pub enum HaulMatrixFeatures {
    Date = 0,
    GearGroup = 1,
    SpeciesGroup = 2,
    VesselLength = 3,
    CatchLocation = 4,
}

impl From<ActiveHaulsFilter> for HaulMatrixFeatures {
    fn from(value: ActiveHaulsFilter) -> Self {
        match value {
            ActiveHaulsFilter::Date => HaulMatrixFeatures::Date,
            ActiveHaulsFilter::GearGroup => HaulMatrixFeatures::GearGroup,
            ActiveHaulsFilter::SpeciesGroup => HaulMatrixFeatures::SpeciesGroup,
            ActiveHaulsFilter::VesselLength => HaulMatrixFeatures::VesselLength,
            ActiveHaulsFilter::CatchLocation => HaulMatrixFeatures::CatchLocation,
        }
    }
}

impl HaulMatrixFeatures {
    fn convert_from_val(&self, val: i32) -> Result<usize, HaulMatrixIndexError> {
        match self {
            HaulMatrixFeatures::Date => {
                let converted = val as usize;
                if converted >= ERS_OLDEST_DATA_MONTHS {
                    Ok(converted - ERS_OLDEST_DATA_MONTHS)
                } else {
                    Err(HaulMatrixIndexError::Date(val))
                }
            }
            HaulMatrixFeatures::GearGroup => GearGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::GearGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixFeatures::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::SpeciesGroup(val))
                .map(|v| v.enum_index()),
            HaulMatrixFeatures::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or(HaulMatrixIndexError::VesselLength(val))
                .map(|v| v.enum_index()),
            HaulMatrixFeatures::CatchLocation => Ok(val as usize),
        }
        .into_report()
    }
    fn size(&self) -> usize {
        match self {
            HaulMatrixFeatures::Date => date_feature_matrix_size(),
            HaulMatrixFeatures::GearGroup => GearGroup::COUNT,
            HaulMatrixFeatures::SpeciesGroup => SpeciesGroup::COUNT,
            HaulMatrixFeatures::VesselLength => VesselLengthGroup::COUNT,
            HaulMatrixFeatures::CatchLocation => NUM_CATCH_LOCATIONS,
        }
    }
    pub fn column_name(&self) -> &'static str {
        match self {
            HaulMatrixFeatures::Date => "matrix_month_bucket",
            HaulMatrixFeatures::GearGroup => "gear_group_id",
            HaulMatrixFeatures::SpeciesGroup => "species_group_id",
            HaulMatrixFeatures::VesselLength => "vessel_length_group",
            HaulMatrixFeatures::CatchLocation => "catch_location_start_matrix_index",
        }
    }
}

pub fn calculate_sum_area_table(
    x_feature: HaulMatrixFeatures,
    y_feature: HaulMatrixFeatures,
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
