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

        if let Some(ranges) = ranges {
            filters.insert(HaulFilter::StartTimestamp(ranges));
        }
        if let Some(ids) = gear_group_ids {
            filters.insert(HaulFilter::GearGroupId(ids));
        }
        if let Some(ids) = species_group_ids {
            filters.insert(HaulFilter::SpeciesGroupIds(ids));
        }
        if let Some(groups) = vessel_length_groups {
            filters.insert(HaulFilter::VesselLengthGroup(groups));
        }
        if let Some(ids) = vessel_ids {
            filters.insert(HaulFilter::FiskeridirVesselId(ids));
        }
        if let Some(locs) = catch_locations {
            filters.insert(HaulFilter::CatchLocations(locs));
        }

        Self {
            filters,
            sorting: sorting.map(HaulSort::from),
            ordering: ordering.unwrap_or_default(),
            pagination: (),
        }
    }
}
