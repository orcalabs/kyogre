use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use error_stack::Result;
use fiskeridir_rs::{GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{FiskeridirVesselId, HaulId, MinMaxBoth, TripSorting, TripsQuery};
use strum_macros::{EnumDiscriminants, EnumIter};

use crate::{
    error::MeilisearchError,
    utils::{join_comma, join_comma_fn, to_nanos},
};

#[derive(Debug, Clone, EnumDiscriminants, strum_macros::Display)]
#[strum_discriminants(
    derive(EnumIter, PartialOrd, Ord, strum_macros::Display),
    strum(serialize_all = "snake_case")
)]
#[strum(serialize_all = "snake_case")]
pub enum TripFilter {
    DeliveryPointIds(Vec<String>),
    Start(DateTime<Utc>),
    End(DateTime<Utc>),
    TotalLivingWeight(MinMaxBoth<f64>),
    GearGroupIds(Vec<GearGroup>),
    SpeciesGroupIds(Vec<SpeciesGroup>),
    FiskeridirLengthGroupId(Vec<VesselLengthGroup>),
    FiskeridirVesselId(Vec<FiskeridirVesselId>),

    HaulIds(HaulId),

    LandingIds(LandingId),
}

#[derive(Debug, Clone, Copy, EnumIter, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum TripSort {
    End,
    TotalLivingWeight,
}

impl TripFilter {
    pub fn filter_str(self) -> Result<String, MeilisearchError> {
        Ok(match self {
            TripFilter::DeliveryPointIds(ids) => format!(
                "{} IN [{}]",
                TripFilterDiscriminants::DeliveryPointIds,
                join_comma(ids)
            ),
            TripFilter::Start(start) => {
                format!("{} >= {}", TripFilterDiscriminants::Start, to_nanos(start)?)
            }
            TripFilter::End(end) => {
                format!("{} <= {}", TripFilterDiscriminants::End, to_nanos(end)?)
            }
            TripFilter::TotalLivingWeight(v) => match v {
                MinMaxBoth::Min(min) => {
                    format!("{} >= {}", TripFilterDiscriminants::TotalLivingWeight, min)
                }
                MinMaxBoth::Max(max) => {
                    format!("{} <= {}", TripFilterDiscriminants::TotalLivingWeight, max)
                }
                MinMaxBoth::Both { min, max } => format!(
                    "{} {} TO {}",
                    TripFilterDiscriminants::TotalLivingWeight,
                    min,
                    max,
                ),
            },
            TripFilter::GearGroupIds(ids) => format!(
                "{} IN [{}]",
                TripFilterDiscriminants::GearGroupIds,
                join_comma_fn(ids, |g| g as i32)
            ),
            TripFilter::SpeciesGroupIds(ids) => format!(
                "{} IN [{}]",
                TripFilterDiscriminants::SpeciesGroupIds,
                join_comma_fn(ids, |s| s as i32)
            ),
            TripFilter::FiskeridirLengthGroupId(ids) => format!(
                "{} IN [{}]",
                TripFilterDiscriminants::FiskeridirLengthGroupId,
                join_comma_fn(ids, |l| l as i32)
            ),
            TripFilter::FiskeridirVesselId(ids) => format!(
                "{} IN [{}]",
                TripFilterDiscriminants::FiskeridirVesselId,
                join_comma_fn(ids, |v| v.0)
            ),
            TripFilter::HaulIds(id) => format!("{} = {}", TripFilterDiscriminants::HaulIds, id),
            TripFilter::LandingIds(id) => {
                format!("{} = {}", TripFilterDiscriminants::LandingIds, id)
            }
        })
    }
}

pub struct Query(BTreeSet<TripFilter>);

impl Query {
    pub fn filter_strs(self) -> Result<Vec<String>, MeilisearchError> {
        self.0
            .into_iter()
            .map(|f| f.filter_str())
            .collect::<Result<_, _>>()
    }
}

impl From<TripsQuery> for Query {
    fn from(value: TripsQuery) -> Self {
        let TripsQuery {
            pagination: _,
            ordering: _,
            sorting: _,
            delivery_points,
            start_date,
            end_date,
            min_weight,
            max_weight,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            fiskeridir_vessel_ids,
        } = value;

        let mut set = BTreeSet::new();

        if let Some(ids) = delivery_points {
            set.insert(TripFilter::DeliveryPointIds(ids));
        }
        if let Some(start) = start_date {
            set.insert(TripFilter::Start(start));
        }
        if let Some(end) = end_date {
            set.insert(TripFilter::End(end));
        }
        if let Some(weight) = MinMaxBoth::new(min_weight, max_weight) {
            set.insert(TripFilter::TotalLivingWeight(weight));
        }
        if let Some(ids) = gear_group_ids {
            set.insert(TripFilter::GearGroupIds(ids));
        }
        if let Some(ids) = species_group_ids {
            set.insert(TripFilter::SpeciesGroupIds(ids));
        }
        if let Some(ids) = vessel_length_groups {
            set.insert(TripFilter::FiskeridirLengthGroupId(ids));
        }
        if let Some(ids) = fiskeridir_vessel_ids {
            set.insert(TripFilter::FiskeridirVesselId(ids));
        }

        Self(set)
    }
}

impl From<&HaulId> for TripFilter {
    fn from(value: &HaulId) -> Self {
        Self::HaulIds(*value)
    }
}

impl From<&LandingId> for TripFilter {
    fn from(value: &LandingId) -> Self {
        Self::LandingIds(value.clone())
    }
}

impl From<TripSorting> for TripSort {
    fn from(value: TripSorting) -> Self {
        match value {
            TripSorting::StopDate => Self::End,
            TripSorting::Weight => Self::TotalLivingWeight,
        }
    }
}

impl PartialEq for TripFilter {
    fn eq(&self, other: &Self) -> bool {
        TripFilterDiscriminants::from(self).eq(&TripFilterDiscriminants::from(other))
    }
}

impl Eq for TripFilter {}

impl PartialOrd for TripFilter {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TripFilter {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        TripFilterDiscriminants::from(self).cmp(&TripFilterDiscriminants::from(other))
    }
}
