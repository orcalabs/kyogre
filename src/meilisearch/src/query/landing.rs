use std::collections::BTreeSet;

use kyogre_core::{Landings, LandingsQuery, Pagination};

use crate::landing::{LandingFilter, LandingSort};

use super::Query;

impl From<LandingsQuery> for Query<LandingFilter, Option<LandingSort>, Pagination<Landings>> {
    fn from(value: LandingsQuery) -> Self {
        let LandingsQuery {
            ordering,
            sorting,
            ranges,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            pagination,
        } = value;

        let mut filters = BTreeSet::new();

        if let Some(ranges) = ranges {
            filters.insert(LandingFilter::LandingTimestamp(ranges));
        }
        if let Some(ids) = gear_group_ids {
            filters.insert(LandingFilter::GearGroupId(ids));
        }
        if let Some(ids) = species_group_ids {
            filters.insert(LandingFilter::SpeciesGroupIds(ids));
        }
        if let Some(groups) = vessel_length_groups {
            filters.insert(LandingFilter::VesselLengthGroup(groups));
        }
        if let Some(ids) = vessel_ids {
            filters.insert(LandingFilter::FiskeridirVesselId(ids));
        }
        if let Some(locs) = catch_locations {
            filters.insert(LandingFilter::CatchLocation(locs));
        }

        Self {
            filters,
            sorting: sorting.map(LandingSort::from),
            ordering: ordering.unwrap_or_default(),
            pagination,
        }
    }
}
