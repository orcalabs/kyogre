use fiskeridir_rs::{Condition, GearGroup, Quality, SpeciesFiskeridirId, VesselLengthGroup};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct PriceQuery {
    #[serde_as(as = "DisplayFromStr")]
    pub length_group: VesselLengthGroup,
    #[serde_as(as = "DisplayFromStr")]
    pub gear_group: GearGroup,
    pub species: SpeciesFiskeridirId,
    #[serde_as(as = "DisplayFromStr")]
    pub condition: Condition,
    #[serde_as(as = "DisplayFromStr")]
    pub quality: Quality,
}
