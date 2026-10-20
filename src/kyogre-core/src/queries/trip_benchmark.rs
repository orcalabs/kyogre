use super::Ordering;
use crate::{DateTimeRange, DateTimeRangeWithDefaultTimeSpan, OptionalDateTimeRange};
use fiskeridir_rs::{CallSign, FiskeridirVesselId, GearGroup, SpeciesGroup, VesselLengthGroup};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug, Clone)]
pub struct TripBenchmarksQuery {
    pub call_sign: CallSign,
    pub range: OptionalDateTimeRange,
    pub ordering: Ordering,
}

#[derive(Debug, Clone)]
pub struct AverageTripBenchmarksQuery {
    pub range: DateTimeRange,
    pub gear_groups: Vec<GearGroup>,
    pub length_group: Option<VesselLengthGroup>,
    pub vessel_ids: Vec<FiskeridirVesselId>,
}

#[derive(Debug, Clone)]
pub struct EeoiQuery {
    pub call_sign: CallSign,
    pub range: OptionalDateTimeRange,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
}

#[derive(Debug, Clone)]
pub struct AverageEeoiQuery {
    pub range: DateTimeRange,
    pub gear_groups: Vec<GearGroup>,
    pub length_group: Option<VesselLengthGroup>,
    pub vessel_ids: Vec<FiskeridirVesselId>,
    pub species_group_id: Option<SpeciesGroup>,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
}

#[derive(Debug, Clone)]
pub struct CarbonIntensityQuery {
    pub call_sign: CallSign,
    pub range: OptionalDateTimeRange,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
}

#[derive(Debug, Clone)]
pub struct AverageCarbonIntensityQuery {
    pub range: DateTimeRange,
    pub gear_groups: Vec<GearGroup>,
    pub length_group: Option<VesselLengthGroup>,
    pub vessel_ids: Vec<FiskeridirVesselId>,
    pub species_group_id: Option<SpeciesGroup>,
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
}

#[serde_as]
#[derive(Default, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct PerVesselBenchmarkParams {
    #[serde(flatten)]
    pub range: DateTimeRangeWithDefaultTimeSpan<30>,
    pub use_following_list: Option<bool>,
    #[serde_as(as = "Option<Vec<DisplayFromStr>>")]
    pub species_group_ids: Option<Vec<SpeciesGroup>>,
}
