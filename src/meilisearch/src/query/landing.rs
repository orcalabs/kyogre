use std::collections::BTreeSet;

use kyogre_core::LandingsQuery;

use crate::landing::{LandingFilter, LandingSort};

use super::Query;

impl From<LandingsQuery> for Query<LandingFilter, Option<LandingSort>, ()> {
    fn from(value: LandingsQuery) -> Self {
        let LandingsQuery {
            ordering,
            sorting,
            ranges,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_ranges,
            vessel_ids,
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
        if let Some(lengths) = vessel_length_ranges {
            filters.insert(LandingFilter::VesselLength(lengths));
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
            pagination: (),
        }
    }
}
