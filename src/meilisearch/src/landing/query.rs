use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use error_stack::Result;
use fiskeridir_rs::{GearGroup, SpeciesGroup};
use kyogre_core::{CatchLocationId, FiskeridirVesselId, LandingsQuery, LandingsSorting, Range};
use strum_macros::{EnumDiscriminants, EnumIter};

use crate::{
    error::MeilisearchError,
    utils::{create_ranges_filter, join_comma, join_comma_fn, to_nanos},
};

#[derive(Debug, Clone, EnumDiscriminants, strum_macros::Display)]
#[strum_discriminants(
    derive(EnumIter, PartialOrd, Ord, strum_macros::Display),
    strum(serialize_all = "snake_case")
)]
#[strum(serialize_all = "snake_case")]
pub enum LandingFilter {
    LandingTimestamp(Vec<Range<DateTime<Utc>>>),
    GearGroupId(Vec<GearGroup>),
    SpeciesGroupIds(Vec<SpeciesGroup>),
    CatchLocation(Vec<CatchLocationId>),
    VesselLength(Vec<Range<f64>>),
    FiskeridirVesselId(Vec<FiskeridirVesselId>),
}

#[derive(Debug, Clone, Copy, EnumIter, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum LandingSort {
    LandingTimestamp,
    TotalLivingWeight,
}

impl LandingFilter {
    pub fn filter_str(self) -> Result<String, MeilisearchError> {
        Ok(match self {
            LandingFilter::LandingTimestamp(ranges) => create_ranges_filter(
                ranges
                    .into_iter()
                    .map(|r| r.try_map(to_nanos))
                    .collect::<Result<Vec<_>, _>>()?,
                LandingFilterDiscriminants::LandingTimestamp,
                LandingFilterDiscriminants::LandingTimestamp,
            ),
            LandingFilter::GearGroupId(ids) => format!(
                "{} IN [{}]",
                LandingFilterDiscriminants::GearGroupId,
                join_comma_fn(ids, |g| g as i32)
            ),
            LandingFilter::SpeciesGroupIds(ids) => format!(
                "{} IN [{}]",
                LandingFilterDiscriminants::SpeciesGroupIds,
                join_comma_fn(ids, |s| s as i32)
            ),
            LandingFilter::VesselLength(lengths) => create_ranges_filter(
                lengths,
                LandingFilterDiscriminants::VesselLength,
                LandingFilterDiscriminants::VesselLength,
            ),
            LandingFilter::FiskeridirVesselId(ids) => format!(
                "{} IN [{}]",
                LandingFilterDiscriminants::FiskeridirVesselId,
                join_comma_fn(ids, |v| v.0)
            ),
            LandingFilter::CatchLocation(locs) => format!(
                "{} IN [{}]",
                LandingFilterDiscriminants::CatchLocation,
                join_comma(locs)
            ),
        })
    }
}

pub struct Query(BTreeSet<LandingFilter>);

impl Query {
    pub fn filter_strs(self) -> Result<Vec<String>, MeilisearchError> {
        self.0
            .into_iter()
            .map(|f| f.filter_str())
            .collect::<Result<_, _>>()
    }
}

impl From<LandingsQuery> for Query {
    fn from(value: LandingsQuery) -> Self {
        let LandingsQuery {
            ordering: _,
            sorting: _,
            ranges,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_ranges,
            vessel_ids,
        } = value;

        let mut set = BTreeSet::new();

        if let Some(ranges) = ranges {
            set.insert(LandingFilter::LandingTimestamp(ranges));
        }
        if let Some(ids) = gear_group_ids {
            set.insert(LandingFilter::GearGroupId(ids));
        }
        if let Some(ids) = species_group_ids {
            set.insert(LandingFilter::SpeciesGroupIds(ids));
        }
        if let Some(lengths) = vessel_length_ranges {
            set.insert(LandingFilter::VesselLength(lengths));
        }
        if let Some(ids) = vessel_ids {
            set.insert(LandingFilter::FiskeridirVesselId(ids));
        }
        if let Some(locs) = catch_locations {
            set.insert(LandingFilter::CatchLocation(locs));
        }

        Self(set)
    }
}

impl From<LandingsSorting> for LandingSort {
    fn from(value: LandingsSorting) -> Self {
        match value {
            LandingsSorting::LandingTimestamp => Self::LandingTimestamp,
            LandingsSorting::LivingWeight => Self::TotalLivingWeight,
        }
    }
}

impl PartialEq for LandingFilter {
    fn eq(&self, other: &Self) -> bool {
        LandingFilterDiscriminants::from(self).eq(&LandingFilterDiscriminants::from(other))
    }
}

impl Eq for LandingFilter {}

impl PartialOrd for LandingFilter {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LandingFilter {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        LandingFilterDiscriminants::from(self).cmp(&LandingFilterDiscriminants::from(other))
    }
}
