use crate::{AisVessel, Mmsi};
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Clone)]
pub struct Vessel {
    pub fiskeridir: FiskeridirVessel,
    pub ais: Option<AisVessel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct FiskeridirVesselId(pub i64);

#[derive(Debug, Clone)]
pub struct FiskeridirVessel {
    pub id: FiskeridirVesselId,
    pub vessel_type_id: Option<u32>,
    pub length_group_id: Option<u32>,
    pub nation_group_id: Option<String>,
    pub nation_id: String,
    pub norwegian_municipality_id: Option<u32>,
    pub norwegian_county_id: Option<u32>,
    pub gross_tonnage_1969: Option<u32>,
    pub gross_tonnage_other: Option<u32>,
    pub call_sign: Option<String>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owner: Option<String>,
    pub engine_building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub building_year: Option<u32>,
    pub rebuilding_year: Option<u32>,
}

impl Vessel {
    pub fn mmsi(&self) -> Option<Mmsi> {
        self.ais.as_ref().map(|v| v.mmsi)
    }
}
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Hash,
    Ord,
    PartialOrd,
)]
#[repr(u8)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum FiskdirVesselNationalityGroup {
    Foreign = 1,
    Norwegian = 2,
    Test = 3,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Serialize_repr,
    Deserialize_repr,
    Hash,
    Ord,
    PartialOrd,
)]
#[repr(u8)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum FiskeridirVesselSource {
    Landings = 1,
    FiskeridirVesselRegister = 2,
}

impl From<fiskeridir_rs::FiskdirVesselNationalityGroup> for FiskdirVesselNationalityGroup {
    fn from(v: fiskeridir_rs::FiskdirVesselNationalityGroup) -> Self {
        match v {
            fiskeridir_rs::FiskdirVesselNationalityGroup::Foreign => Self::Foreign,
            fiskeridir_rs::FiskdirVesselNationalityGroup::Norwegian => Self::Norwegian,
            fiskeridir_rs::FiskdirVesselNationalityGroup::Test => Self::Test,
        }
    }
}

impl From<FiskdirVesselNationalityGroup> for fiskeridir_rs::FiskdirVesselNationalityGroup {
    fn from(v: FiskdirVesselNationalityGroup) -> Self {
        match v {
            FiskdirVesselNationalityGroup::Foreign => Self::Foreign,
            FiskdirVesselNationalityGroup::Norwegian => Self::Norwegian,
            FiskdirVesselNationalityGroup::Test => Self::Test,
        }
    }
}
