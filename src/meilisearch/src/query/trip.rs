use std::collections::BTreeSet;

use kyogre_core::{MinMaxBoth, Pagination, Trips, TripsQuery};

use crate::trip::{TripFilter, TripSort};

use super::Query;

impl From<TripsQuery> for Query<TripFilter, TripSort, Pagination<Trips>> {
    fn from(value: TripsQuery) -> Self {
        let TripsQuery {
            pagination,
            ordering,
            sorting,
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

        let mut filters = BTreeSet::new();

        if let Some(ids) = delivery_points {
            filters.insert(TripFilter::DeliveryPointIds(ids));
        }
        if let Some(start) = start_date {
            filters.insert(TripFilter::Start(start));
        }
        if let Some(end) = end_date {
            filters.insert(TripFilter::End(end));
        }
        if let Some(weight) = MinMaxBoth::new(min_weight, max_weight) {
            filters.insert(TripFilter::TotalLivingWeight(weight));
        }
        if let Some(ids) = gear_group_ids {
            filters.insert(TripFilter::GearGroupIds(ids));
        }
        if let Some(ids) = species_group_ids {
            filters.insert(TripFilter::SpeciesGroupIds(ids));
        }
        if let Some(ids) = vessel_length_groups {
            filters.insert(TripFilter::FiskeridirLengthGroupId(ids));
        }
        if let Some(ids) = fiskeridir_vessel_ids {
            filters.insert(TripFilter::FiskeridirVesselId(ids));
        }

        Self {
            filters,
            sorting: sorting.into(),
            ordering,
            pagination,
        }
    }
}
