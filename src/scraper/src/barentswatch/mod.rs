use std::sync::Arc;

use http_client::HttpClient;
use serde::Deserialize;

mod fishing_facility;
mod fishing_facility_historic;

pub use fishing_facility::*;
pub use fishing_facility_historic::*;

pub struct BarentswatchSource {
    pub client: Arc<HttpClient>,
}

impl BarentswatchSource {
    pub fn new(client: Arc<HttpClient>) -> Self {
        Self { client }
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
    Sensorcable,
    #[serde(alias = "UNK")]
    Unknown,
    Seismic,
    Mooring,
    PlannedCableLaying,
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
            FishingFacilityToolType::Sensorcable => Self::Sensorcable,
            FishingFacilityToolType::Unknown => Self::Unknown,
            FishingFacilityToolType::Seismic => Self::Seismic,
            FishingFacilityToolType::Mooring => Self::Mooring,
            FishingFacilityToolType::PlannedCableLaying => Self::PlannedCableLaying,
        }
    }
}
