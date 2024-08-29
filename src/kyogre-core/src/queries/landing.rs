use crate::{
    matrix_index_error::ValueSnafu, CatchLocationId, FiskeridirVesselId, MatrixIndexError,
    Ordering, Range, LANDING_OLDEST_DATA_MONTHS, NUM_CATCH_LOCATIONS,
};
use chrono::{DateTime, Datelike, Months, Utc};
use enum_index::EnumIndex;
use enum_index_derive::EnumIndex;
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumCount, EnumIter, EnumString};

use super::compute_sum_area_table;

#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
pub enum ActiveLandingFilter {
    Date = 1,
    GearGroup = 2,
    SpeciesGroup = 3,
    VesselLength = 4,
}

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[derive(
    Default, Debug, Clone, Copy, Deserialize, Serialize, strum::Display, AsRefStr, EnumString,
)]
pub enum LandingsSorting {
    #[serde(
        alias = "landingTimestamp",
        alias = "LandingTimestamp",
        alias = "LANDING_TIMESTAMP"
    )]
    #[default]
    LandingTimestamp = 1,
    #[serde(
        alias = "livingWeight",
        alias = "LivingWeight",
        alias = "LIVING_WEIGHT"
    )]
    LivingWeight = 2,
}

#[derive(Default, Debug, Clone)]
pub struct LandingsQuery {
    pub ranges: Option<Vec<Range<DateTime<Utc>>>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub sorting: Option<LandingsSorting>,
    pub ordering: Option<Ordering>,
}

#[derive(Debug, Clone)]
pub struct LandingMatrixQuery {
    pub months: Option<Vec<u32>>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
    pub gear_group_ids: Option<Vec<GearGroup>>,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
    pub vessel_length_groups: Option<Vec<VesselLengthGroup>>,
    pub vessel_ids: Option<Vec<FiskeridirVesselId>>,
    pub active_filter: ActiveLandingFilter,
}

#[derive(Debug, Clone)]
pub struct LandingMatrixQueryOutput {
    pub sum_living: u64,
    pub x_index: i32,
    pub y_index: i32,
}

#[derive(Debug, Copy, Clone, EnumIter, PartialEq, Eq, Display)]
pub enum LandingMatrixes {
    Date,
    GearGroup,
    SpeciesGroup,
    VesselLength,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LandingMatrixXFeature {
    Date = 0,
    GearGroup = 1,
    SpeciesGroup = 2,
    VesselLength = 3,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LandingMatrixYFeature {
    Date = 0,
    GearGroup = 1,
    SpeciesGroup = 2,
    VesselLength = 3,
    CatchLocation = 4,
}

impl LandingMatrixXFeature {
    pub fn column_name(&self) -> &'static str {
        match self {
            LandingMatrixXFeature::Date => "matrix_month_bucket",
            LandingMatrixXFeature::GearGroup => "gear_group_id",
            LandingMatrixXFeature::SpeciesGroup => "species_group_id",
            LandingMatrixXFeature::VesselLength => "vessel_length_group",
        }
    }
    fn convert_from_val(&self, val: i32) -> Result<usize, MatrixIndexError> {
        match self {
            LandingMatrixXFeature::Date => {
                let converted = val as usize;
                if converted >= LANDING_OLDEST_DATA_MONTHS {
                    Ok(converted - LANDING_OLDEST_DATA_MONTHS)
                } else {
                    ValueSnafu { val }.fail()
                }
            }
            LandingMatrixXFeature::GearGroup => GearGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            LandingMatrixXFeature::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            LandingMatrixXFeature::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
        }
    }
    fn size(&self) -> usize {
        match self {
            LandingMatrixXFeature::Date => landing_date_feature_matrix_size(),
            LandingMatrixXFeature::GearGroup => GearGroup::COUNT,
            LandingMatrixXFeature::SpeciesGroup => SpeciesGroup::COUNT,
            LandingMatrixXFeature::VesselLength => VesselLengthGroup::COUNT,
        }
    }
}

impl LandingMatrixYFeature {
    pub fn column_name(&self) -> &'static str {
        match self {
            LandingMatrixYFeature::Date => "matrix_month_bucket",
            LandingMatrixYFeature::GearGroup => "gear_group_id",
            LandingMatrixYFeature::SpeciesGroup => "species_group_id",
            LandingMatrixYFeature::VesselLength => "vessel_length_group",
            LandingMatrixYFeature::CatchLocation => "catch_location_matrix_index",
        }
    }
    fn convert_from_val(&self, val: i32) -> Result<usize, MatrixIndexError> {
        match self {
            LandingMatrixYFeature::Date => {
                let converted = val as usize;
                if converted >= LANDING_OLDEST_DATA_MONTHS {
                    Ok(converted - LANDING_OLDEST_DATA_MONTHS)
                } else {
                    ValueSnafu { val }.fail()
                }
            }
            LandingMatrixYFeature::GearGroup => GearGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            LandingMatrixYFeature::SpeciesGroup => SpeciesGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            LandingMatrixYFeature::VesselLength => VesselLengthGroup::from_i32(val)
                .ok_or_else(|| ValueSnafu { val }.build())
                .map(|v| v.enum_index()),
            LandingMatrixYFeature::CatchLocation => Ok(val as usize),
        }
    }

    fn size(&self) -> usize {
        match self {
            LandingMatrixYFeature::Date => landing_date_feature_matrix_size(),
            LandingMatrixYFeature::GearGroup => GearGroup::COUNT,
            LandingMatrixYFeature::SpeciesGroup => SpeciesGroup::COUNT,
            LandingMatrixYFeature::VesselLength => VesselLengthGroup::COUNT,
            LandingMatrixYFeature::CatchLocation => NUM_CATCH_LOCATIONS,
        }
    }
}

impl LandingMatrixes {
    pub fn size(&self) -> usize {
        match self {
            LandingMatrixes::Date => landing_date_feature_matrix_size(),
            LandingMatrixes::GearGroup => GearGroup::COUNT,
            LandingMatrixes::SpeciesGroup => SpeciesGroup::COUNT,
            LandingMatrixes::VesselLength => VesselLengthGroup::COUNT,
        }
    }
}

fn landing_date_feature_matrix_size() -> usize {
    let diff = chrono::Utc::now() - Months::new(LANDING_OLDEST_DATA_MONTHS as u32);
    (diff.month() as i32 + (diff.year() * 12)) as usize
}

pub fn landing_date_feature_matrix_index(ts: &DateTime<Utc>) -> usize {
    ts.year() as usize * 12 + ts.month0() as usize - LANDING_OLDEST_DATA_MONTHS
}

impl From<ActiveLandingFilter> for LandingMatrixes {
    fn from(value: ActiveLandingFilter) -> Self {
        match value {
            ActiveLandingFilter::Date => LandingMatrixes::Date,
            ActiveLandingFilter::GearGroup => LandingMatrixes::GearGroup,
            ActiveLandingFilter::SpeciesGroup => LandingMatrixes::SpeciesGroup,
            ActiveLandingFilter::VesselLength => LandingMatrixes::VesselLength,
        }
    }
}

impl PartialEq<ActiveLandingFilter> for LandingMatrixXFeature {
    fn eq(&self, other: &ActiveLandingFilter) -> bool {
        let tmp: LandingMatrixXFeature = (*other).into();
        self.eq(&tmp)
    }
}

impl From<ActiveLandingFilter> for LandingMatrixXFeature {
    fn from(value: ActiveLandingFilter) -> Self {
        match value {
            ActiveLandingFilter::Date => LandingMatrixXFeature::Date,
            ActiveLandingFilter::GearGroup => LandingMatrixXFeature::GearGroup,
            ActiveLandingFilter::SpeciesGroup => LandingMatrixXFeature::SpeciesGroup,
            ActiveLandingFilter::VesselLength => LandingMatrixXFeature::VesselLength,
        }
    }
}

impl From<ActiveLandingFilter> for LandingMatrixYFeature {
    fn from(value: ActiveLandingFilter) -> Self {
        match value {
            ActiveLandingFilter::Date => LandingMatrixYFeature::Date,
            ActiveLandingFilter::GearGroup => LandingMatrixYFeature::GearGroup,
            ActiveLandingFilter::SpeciesGroup => LandingMatrixYFeature::SpeciesGroup,
            ActiveLandingFilter::VesselLength => LandingMatrixYFeature::VesselLength,
        }
    }
}

impl PartialEq<LandingMatrixes> for ActiveLandingFilter {
    fn eq(&self, other: &LandingMatrixes) -> bool {
        match self {
            ActiveLandingFilter::Date => matches!(other, LandingMatrixes::Date),
            ActiveLandingFilter::GearGroup => matches!(other, LandingMatrixes::GearGroup),
            ActiveLandingFilter::SpeciesGroup => matches!(other, LandingMatrixes::SpeciesGroup),
            ActiveLandingFilter::VesselLength => matches!(other, LandingMatrixes::VesselLength),
        }
    }
}

pub fn calculate_landing_sum_area_table(
    x_feature: LandingMatrixXFeature,
    y_feature: LandingMatrixYFeature,
    data: Vec<LandingMatrixQueryOutput>,
) -> Result<Vec<u64>, MatrixIndexError> {
    let height = y_feature.size();
    let width = x_feature.size();

    let mut matrix: Vec<u64> = vec![0; width * height];

    for d in data {
        let x = x_feature.convert_from_val(d.x_index)?;
        let y = y_feature.convert_from_val(d.y_index)?;

        matrix[(y * width) + x] = d.sum_living;
    }

    compute_sum_area_table(&mut matrix, width);

    Ok(matrix)
}
