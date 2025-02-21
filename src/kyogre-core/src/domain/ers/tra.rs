use chrono::{DateTime, Utc};
use fiskeridir_rs::CallSign;
use fiskeridir_rs::{FiskeridirVesselId, SpeciesGroup};
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;

use super::ErsQuantumType;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct Tra {
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub reloading_timestamp: Option<DateTime<Utc>>,
    pub message_timestamp: DateTime<Utc>,
    pub catches: Vec<TraCatch>,
    pub reload_to_fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub reload_from_fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub reload_to_call_sign: Option<CallSign>,
    pub reload_from_call_sign: Option<CallSign>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub struct TraCatch {
    pub living_weight: i32,
    #[serde_as(as = "DisplayFromStr")]
    pub species_group_id: SpeciesGroup,
    pub catch_quantum: ErsQuantumType,
}
