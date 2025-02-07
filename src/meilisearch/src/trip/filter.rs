use crate::{
    error::Result,
    query::Filter,
    utils::{join_comma, join_comma_fn, to_nanos},
};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::{FiskeridirVesselId, MinMaxBoth, TripId, TripSorting};
use strum::{EnumDiscriminants, EnumIter};

#[derive(Debug, Clone, EnumDiscriminants, strum::Display)]
#[strum_discriminants(
    derive(EnumIter, PartialOrd, Ord, strum::Display),
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
    TripId(Vec<TripId>),
}

#[derive(Debug, Clone, Copy, EnumIter, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum TripSort {
    End,
    TotalLivingWeight,
}

impl Filter for TripFilter {
    fn filter_str(self) -> Result<String> {
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
                join_comma(ids)
            ),
            TripFilter::TripId(ids) => format!(
                "{} IN [{}]",
                TripFilterDiscriminants::TripId,
                join_comma(ids)
            ),
        })
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
