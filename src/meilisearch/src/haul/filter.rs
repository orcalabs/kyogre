use chrono::{DateTime, Utc};
use error_stack::Result;
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, HaulsSorting, MinMaxBoth, Range};
use strum_macros::{EnumDiscriminants, EnumIter};

use crate::{
    error::MeilisearchError,
    query::Filter,
    utils::{create_ranges_filter, join_comma, join_comma_fn, to_nanos},
};

mod never {
    #[derive(Debug, Clone)]
    pub struct Never(());
}

#[derive(Debug, Clone, EnumDiscriminants, strum_macros::Display)]
#[strum_discriminants(
    derive(EnumIter, PartialOrd, Ord, strum_macros::Display),
    strum(serialize_all = "snake_case")
)]
#[strum(serialize_all = "snake_case")]
pub enum HaulFilter {
    StartTimestamp(Vec<Range<DateTime<Utc>>>),
    // `StopTimestamp` is defined here because it needs to be a filterable attribute of hauls,
    // and it is unused because it is always used in conjunction with `StartTimestamp`.
    #[allow(dead_code)]
    StopTimestamp(never::Never),
    #[strum_discriminants(strum(serialize = "wind_speed_10m"))]
    WindSpeed(MinMaxBoth<f64>),
    #[strum_discriminants(strum(serialize = "air_temperature_2m"))]
    AirTemperature(MinMaxBoth<f64>),
    GearGroupId(Vec<GearGroup>),
    SpeciesGroupIds(Vec<SpeciesGroup>),
    CatchLocations(Vec<CatchLocationId>),
    VesselLength(Vec<Range<f64>>),
    FiskeridirVesselId(Vec<FiskeridirVesselId>),
}

#[derive(Debug, Clone, Copy, EnumIter, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum HaulSort {
    StartTimestamp,
    StopTimestamp,
    TotalLivingWeight,
}

impl Filter for HaulFilter {
    fn filter_str(self) -> Result<String, MeilisearchError> {
        Ok(match self {
            HaulFilter::StartTimestamp(ranges) => create_ranges_filter(
                ranges
                    .into_iter()
                    .map(|r| r.try_map(to_nanos))
                    .collect::<Result<Vec<_>, _>>()?,
                HaulFilterDiscriminants::StopTimestamp,
                HaulFilterDiscriminants::StartTimestamp,
            ),
            HaulFilter::StopTimestamp(_) => unreachable!(),
            HaulFilter::WindSpeed(v) => match v {
                MinMaxBoth::Min(min) => {
                    format!("{} >= {}", HaulFilterDiscriminants::WindSpeed, min)
                }
                MinMaxBoth::Max(max) => {
                    format!("{} <= {}", HaulFilterDiscriminants::WindSpeed, max)
                }
                MinMaxBoth::Both { min, max } => {
                    format!("{} {} TO {}", HaulFilterDiscriminants::WindSpeed, min, max,)
                }
            },
            HaulFilter::AirTemperature(v) => match v {
                MinMaxBoth::Min(min) => {
                    format!("{} >= {}", HaulFilterDiscriminants::AirTemperature, min)
                }
                MinMaxBoth::Max(max) => {
                    format!("{} <= {}", HaulFilterDiscriminants::AirTemperature, max)
                }
                MinMaxBoth::Both { min, max } => format!(
                    "{} {} TO {}",
                    HaulFilterDiscriminants::AirTemperature,
                    min,
                    max,
                ),
            },
            HaulFilter::GearGroupId(ids) => format!(
                "{} IN [{}]",
                HaulFilterDiscriminants::GearGroupId,
                join_comma_fn(ids, |g| g as i32)
            ),
            HaulFilter::SpeciesGroupIds(ids) => format!(
                "{} IN [{}]",
                HaulFilterDiscriminants::SpeciesGroupIds,
                join_comma_fn(ids, |s| s as i32)
            ),
            HaulFilter::VesselLength(lengths) => create_ranges_filter(
                lengths,
                HaulFilterDiscriminants::VesselLength,
                HaulFilterDiscriminants::VesselLength,
            ),
            HaulFilter::FiskeridirVesselId(ids) => format!(
                "{} IN [{}]",
                HaulFilterDiscriminants::FiskeridirVesselId,
                join_comma_fn(ids, |v| v.0)
            ),
            HaulFilter::CatchLocations(locs) => format!(
                "{} IN [{}]",
                HaulFilterDiscriminants::CatchLocations,
                join_comma(locs)
            ),
        })
    }
}

impl From<HaulsSorting> for HaulSort {
    fn from(value: HaulsSorting) -> Self {
        match value {
            HaulsSorting::StartDate => Self::StartTimestamp,
            HaulsSorting::StopDate => Self::StopTimestamp,
            HaulsSorting::Weight => Self::TotalLivingWeight,
        }
    }
}

impl PartialEq for HaulFilter {
    fn eq(&self, other: &Self) -> bool {
        HaulFilterDiscriminants::from(self).eq(&HaulFilterDiscriminants::from(other))
    }
}

impl Eq for HaulFilter {}

impl PartialOrd for HaulFilter {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HaulFilter {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        HaulFilterDiscriminants::from(self).cmp(&HaulFilterDiscriminants::from(other))
    }
}
