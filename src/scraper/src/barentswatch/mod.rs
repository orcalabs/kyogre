use serde::Deserialize;
use std::sync::Arc;

use crate::wrapped_http_client::WrappedHttpClient;

mod fishing_facility;

pub use fishing_facility::*;

pub struct BarentswatchSource {
    pub client: Arc<WrappedHttpClient>,
}

impl BarentswatchSource {
    pub fn new(client: Arc<WrappedHttpClient>) -> BarentswatchSource {
        BarentswatchSource { client }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum FishingFacilityToolType {
    Undefined,
    Crabpot,
    Danpurseine,
    Nets,
    Longline,
    Generic,
    Sensorbuoy,
}

impl From<FishingFacilityToolType> for kyogre_core::FishingFacilityToolType {
    fn from(v: FishingFacilityToolType) -> Self {
        match v {
            FishingFacilityToolType::Undefined => Self::Undefined,
            FishingFacilityToolType::Crabpot => Self::Crabpot,
            FishingFacilityToolType::Danpurseine => Self::Danpurseine,
            FishingFacilityToolType::Nets => Self::Nets,
            FishingFacilityToolType::Longline => Self::Longline,
            FishingFacilityToolType::Generic => Self::Generic,
            FishingFacilityToolType::Sensorbuoy => Self::Sensorbuoy,
        }
    }
}
