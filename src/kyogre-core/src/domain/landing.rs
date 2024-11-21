use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    CallSign, DeliveryPointId, Gear, GearGroup, LandingId, SpeciesGroup, VesselLengthGroup,
};
use serde::{Deserialize, Serialize};

use crate::{
    ActiveLandingFilter, CatchLocationId, FiskeridirVesselId, LandingMatrixXFeature,
    LandingMatrixYFeature,
};

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
    pub id: LandingId,
    pub landing_timestamp: DateTime<Utc>,
    pub catch_location: Option<CatchLocationId>,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub delivery_point_id: Option<DeliveryPointId>,
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_call_sign: Option<CallSign>,
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

impl LandingMatrix {
    pub fn is_empty(&self) -> bool {
        let Self {
            dates,
            length_group,
            gear_group,
            species_group,
        } = self;

        dates.iter().all(|v| *v == 0)
            && length_group.iter().all(|v| *v == 0)
            && gear_group.iter().all(|v| *v == 0)
            && species_group.iter().all(|v| *v == 0)
    }

    pub fn empty(active_filter: ActiveLandingFilter) -> LandingMatrix {
        let x_feature: LandingMatrixXFeature = active_filter.into();
        let dates_size = if x_feature == LandingMatrixXFeature::Date {
            LandingMatrixYFeature::Date.size() * LandingMatrixYFeature::CatchLocation.size()
        } else {
            LandingMatrixYFeature::Date.size() * x_feature.size()
        };

        let length_group_size = if x_feature == LandingMatrixXFeature::VesselLength {
            LandingMatrixYFeature::VesselLength.size() * LandingMatrixYFeature::CatchLocation.size()
        } else {
            LandingMatrixYFeature::VesselLength.size() * x_feature.size()
        };

        let gear_group_size = if x_feature == LandingMatrixXFeature::GearGroup {
            LandingMatrixYFeature::GearGroup.size() * LandingMatrixYFeature::CatchLocation.size()
        } else {
            LandingMatrixYFeature::GearGroup.size() * x_feature.size()
        };

        let species_group_size = if x_feature == LandingMatrixXFeature::SpeciesGroup {
            LandingMatrixYFeature::SpeciesGroup.size() * LandingMatrixYFeature::CatchLocation.size()
        } else {
            LandingMatrixYFeature::SpeciesGroup.size() * x_feature.size()
        };

        Self {
            dates: vec![0; dates_size],
            length_group: vec![0; length_group_size],
            gear_group: vec![0; gear_group_size],
            species_group: vec![0; species_group_size],
        }
    }
}
