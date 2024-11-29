use fiskeridir_rs::{GearGroup, SpeciesGroup, VesselLengthGroup};
use kyogre_core::CatchLocationId;
use kyogre_core::*;
use std::fmt::{Display, Formatter};

pub struct LandingFilters<'a> {
    filters: Vec<LandingFilter<'a>>,
}

pub struct HaulFilters<'a> {
    filters: Vec<HaulFilter<'a>>,
}

enum LandingFilter<'a> {
    MonthBuckets(&'a [u32]),
    CatchLocations(&'a [CatchLocationId]),
    GearGroup(&'a [GearGroup]),
    SpeciesGroup(&'a [SpeciesGroup]),
    VesselLengthGroups(&'a [VesselLengthGroup]),
    VesselIds(&'a [FiskeridirVesselId]),
}

enum HaulFilter<'a> {
    MajoritySpeciesGroup(bool),
    BycatchPercentage(f64),
    MonthBuckets(&'a [u32]),
    CatchLocations(&'a [CatchLocationId]),
    GearGroup(&'a [GearGroup]),
    SpeciesGroup(&'a [SpeciesGroup]),
    VesselLengthGroups(&'a [VesselLengthGroup]),
    VesselIds(&'a [FiskeridirVesselId]),
}

impl std::fmt::Display for HaulFilter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaulFilter::CatchLocations(vals) => write_array_filter(f, "catch_location_id", vals),
            HaulFilter::MonthBuckets(vals) => write_array_filter(f, "matrix_month_bucket", vals),
            HaulFilter::GearGroup(vals) => write_array_filter(f, "gear_group_id", vals),
            HaulFilter::SpeciesGroup(vals) => write_array_filter(f, "species_group_id", vals),
            HaulFilter::VesselLengthGroups(vals) => {
                write_array_filter(f, "vessel_length_group", vals)
            }
            HaulFilter::VesselIds(vals) => write_array_filter(f, "fiskeridir_vessel_id", vals),
            HaulFilter::MajoritySpeciesGroup(v) => {
                f.write_fmt(format_args!("is_majority_species_group_of_haul = {} ", v))
            }
            HaulFilter::BycatchPercentage(v) => f.write_fmt(format_args!(
                "species_group_weight_percentage_of_haul >= {} ",
                v
            )),
        }
    }
}

impl std::fmt::Display for LandingFilter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LandingFilter::CatchLocations(vals) => write_array_filter(f, "catch_location_id", vals),
            LandingFilter::MonthBuckets(vals) => write_array_filter(f, "matrix_month_bucket", vals),
            LandingFilter::GearGroup(vals) => write_array_filter(f, "gear_group_id", vals),
            LandingFilter::SpeciesGroup(vals) => write_array_filter(f, "species_group_id", vals),
            LandingFilter::VesselLengthGroups(vals) => {
                write_array_filter(f, "vessel_length_group", vals)
            }
            LandingFilter::VesselIds(vals) => write_array_filter(f, "fiskeridir_vessel_id", vals),
        }
    }
}

trait FilterValue {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result;
}

impl FilterValue for GearGroup {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", *self as u32))
    }
}

impl FilterValue for SpeciesGroup {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", *self as u32))
    }
}

impl FilterValue for VesselLengthGroup {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", *self as u32))
    }
}

impl FilterValue for u32 {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}

impl FilterValue for CatchLocationId {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("'{}'", self))
    }
}

impl FilterValue for FiskeridirVesselId {
    fn filter_fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt(f)
    }
}

impl<'a> HaulFilters<'a> {
    pub fn query_string(self) -> String {
        query_string(self.filters)
    }
    pub fn new(
        query: &'a HaulsMatrixQuery,
        x_feature: HaulMatrixXFeature,
        y_feature: HaulMatrixYFeature,
    ) -> Self {
        let mut filters = Vec::new();
        let HaulsMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            bycatch_percentage,
            majority_species_group,
            active_filter: _,
        } = query;

        if *majority_species_group {
            filters.push(HaulFilter::MajoritySpeciesGroup(*majority_species_group));
        }

        if let Some(bycatch_percentage) = bycatch_percentage {
            filters.push(HaulFilter::BycatchPercentage(*bycatch_percentage));
        }

        if !months.is_empty()
            && y_feature != HaulMatrixYFeature::Date
            && x_feature != HaulMatrixXFeature::Date
        {
            filters.push(HaulFilter::MonthBuckets(months));
        }

        if !catch_locations.is_empty() && y_feature != HaulMatrixYFeature::CatchLocation {
            filters.push(HaulFilter::CatchLocations(catch_locations));
        }
        if !gear_group_ids.is_empty()
            && y_feature != HaulMatrixYFeature::GearGroup
            && x_feature != HaulMatrixXFeature::GearGroup
        {
            filters.push(HaulFilter::GearGroup(gear_group_ids));
        }
        if !species_group_ids.is_empty()
            && y_feature != HaulMatrixYFeature::SpeciesGroup
            && x_feature != HaulMatrixXFeature::SpeciesGroup
        {
            filters.push(HaulFilter::SpeciesGroup(species_group_ids));
        }
        if !vessel_length_groups.is_empty()
            && y_feature != HaulMatrixYFeature::VesselLength
            && x_feature != HaulMatrixXFeature::VesselLength
        {
            filters.push(HaulFilter::VesselLengthGroups(vessel_length_groups));
        }
        if !vessel_ids.is_empty() {
            filters.push(HaulFilter::VesselIds(vessel_ids));
        }

        HaulFilters { filters }
    }
}
impl<'a> LandingFilters<'a> {
    pub fn query_string(self) -> String {
        query_string(self.filters)
    }

    pub fn new(
        query: &'a LandingMatrixQuery,
        x_feature: LandingMatrixXFeature,
        y_feature: LandingMatrixYFeature,
    ) -> Self {
        let mut filters = Vec::new();
        let LandingMatrixQuery {
            months,
            catch_locations,
            gear_group_ids,
            species_group_ids,
            vessel_length_groups,
            vessel_ids,
            active_filter: _,
        } = query;

        if !months.is_empty()
            && y_feature != LandingMatrixYFeature::Date
            && x_feature != LandingMatrixXFeature::Date
        {
            filters.push(LandingFilter::MonthBuckets(months));
        }

        if !catch_locations.is_empty() && y_feature != LandingMatrixYFeature::CatchLocation {
            filters.push(LandingFilter::CatchLocations(catch_locations));
        }
        if !gear_group_ids.is_empty()
            && y_feature != LandingMatrixYFeature::GearGroup
            && x_feature != LandingMatrixXFeature::GearGroup
        {
            filters.push(LandingFilter::GearGroup(gear_group_ids));
        }
        if !species_group_ids.is_empty()
            && y_feature != LandingMatrixYFeature::SpeciesGroup
            && x_feature != LandingMatrixXFeature::SpeciesGroup
        {
            filters.push(LandingFilter::SpeciesGroup(species_group_ids));
        }
        if !vessel_length_groups.is_empty()
            && y_feature != LandingMatrixYFeature::VesselLength
            && x_feature != LandingMatrixXFeature::VesselLength
        {
            filters.push(LandingFilter::VesselLengthGroups(vessel_length_groups));
        }
        if !vessel_ids.is_empty() {
            filters.push(LandingFilter::VesselIds(vessel_ids));
        }

        LandingFilters { filters }
    }
}

fn write_array_filter<T: FilterValue>(
    f: &mut std::fmt::Formatter<'_>,
    column_name: &'static str,
    vals: &[T],
) -> std::fmt::Result {
    f.write_fmt(format_args!("{column_name} = ANY(["))?;
    let len = vals.len();
    for (i, v) in vals.iter().enumerate() {
        v.filter_fmt(f)?;
        if i != len - 1 {
            f.write_str(", ")?;
        }
    }
    f.write_str("]) ")
}

fn query_string<T: Display>(values: Vec<T>) -> String {
    let mut query = String::new();
    let len = values.len();
    for (i, f) in values.into_iter().enumerate() {
        if i == 0 {
            query.push_str("where ");
        } else {
            query.push_str("and ");
        }
        query.push_str(&format!("{f}"));

        if i == len - 1 {
            query.push(' ');
        }
    }

    query
}
