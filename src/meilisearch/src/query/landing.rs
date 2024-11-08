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

        if !ranges.is_empty() {
            filters.insert(LandingFilter::LandingTimestamp(ranges));
        }
        if !gear_group_ids.is_empty() {
            filters.insert(LandingFilter::GearGroupId(gear_group_ids));
        }
        if !species_group_ids.is_empty() {
            filters.insert(LandingFilter::SpeciesGroupIds(species_group_ids));
        }
        if !vessel_length_groups.is_empty() {
            filters.insert(LandingFilter::VesselLengthGroup(vessel_length_groups));
        }
        if !vessel_ids.is_empty() {
            filters.insert(LandingFilter::FiskeridirVesselId(vessel_ids));
        }
        if !catch_locations.is_empty() {
            filters.insert(LandingFilter::CatchLocation(catch_locations));
        }

        Self {
            filters,
            sorting: sorting.map(LandingSort::from),
            ordering: ordering.unwrap_or_default(),
            pagination,
        }
    }
}
