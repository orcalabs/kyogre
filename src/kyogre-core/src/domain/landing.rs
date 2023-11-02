use chrono::{DateTime, Utc};
use fiskeridir_rs::{DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup};
use serde::{Deserialize, Serialize};

use crate::{CatchLocationId, FiskeridirVesselId};

pub static LANDING_OLDEST_DATA_MONTHS: usize = 1999 * 12;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct LandingMatrix {
    pub dates: Vec<u64>,
    pub length_group: Vec<u64>,
    pub gear_group: Vec<u64>,
    pub species_group: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    pub landing_id: LandingId,
    pub landing_timestamp: DateTime<Utc>,
    pub catch_location: Option<CatchLocationId>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub delivery_point_id: Option<DeliveryPointId>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_call_sign: Option<String>,
    pub vessel_name: Option<String>,
    pub vessel_length: Option<f64>,
    pub vessel_length_group: VesselLengthGroup,
    pub total_living_weight: f64,
    pub total_product_weight: f64,
    pub total_gross_weight: f64,
    pub catches: Vec<LandingCatch>,
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandingCatch {
    pub living_weight: f64,
    pub gross_weight: f64,
    pub product_weight: f64,
    pub species_fiskeridir_id: i32,
    pub species_group_id: SpeciesGroup,
}
