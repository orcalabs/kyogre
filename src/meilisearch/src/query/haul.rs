use crate::haul::{HaulFilter, HaulSort};
use kyogre_core::HaulsQuery;
use std::collections::BTreeSet;

use super::Query;

impl From<HaulsQuery> for Query<HaulFilter, Option<HaulSort>, ()> {
    fn from(value: HaulsQuery) -> Self {
        let HaulsQuery {
            ordering,
            sorting,
            ranges,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
        } = value;

        let mut filters = BTreeSet::new();

        if !ranges.is_empty() {
            filters.insert(HaulFilter::StartTimestamp(ranges));
        }
        if !gear_group_ids.is_empty() {
            filters.insert(HaulFilter::GearGroupId(gear_group_ids));
        }
        if !species_group_ids.is_empty() {
            filters.insert(HaulFilter::SpeciesGroupIds(species_group_ids));
        }
        if !vessel_length_groups.is_empty() {
            filters.insert(HaulFilter::VesselLengthGroup(vessel_length_groups));
        }
        if !vessel_ids.is_empty() {
            filters.insert(HaulFilter::FiskeridirVesselId(vessel_ids));
        }
        if !catch_locations.is_empty() {
            filters.insert(HaulFilter::CatchLocations(catch_locations));
        }

        Self {
            filters,
            sorting: sorting.map(HaulSort::from),
            ordering: ordering.unwrap_or_default(),
            pagination: (),
        }
    }
}
