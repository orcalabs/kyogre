use crate::mean::Mean;
use crate::{AisVessel, Mmsi, TripAssemblerId};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    CallSign, FiskeridirVesselId, GearGroup, RegisterVesselOwner, SpeciesGroup, VesselLengthGroup,
};
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{DisplayFromStr, serde_as};
use strum::{AsRefStr, Display, EnumString};

mod benchmark;

pub use benchmark::*;

use super::VesselCurrentTrip;

pub const HP_TO_KW: f64 = 0.745699872;
pub static TEST_SIGNED_IN_VESSEL_CALLSIGN: &str = "LK17";

/// These have been observed in data from Fiskeridirektoratet, and we assume that they are safe to
/// ignore.
pub static IGNORED_CONFLICT_CALL_SIGNS: &[&str] = &["00000000", "0"];

#[serde_as]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVessel {
    pub engine_power: Option<u32>,
    pub engine_building_year: Option<u32>,
    pub auxiliary_engine_power: Option<u32>,
    pub auxiliary_engine_building_year: Option<u32>,
    pub boiler_engine_power: Option<u32>,
    pub boiler_engine_building_year: Option<u32>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub engine_type: Option<EngineType>,
    pub engine_rpm: Option<u32>,
    pub degree_of_electrification: Option<f64>,
    pub service_speed: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LiveFuelVessel {
    pub mmsi: Mmsi,
    pub vessel_id: FiskeridirVesselId,
    pub current_trip_start: Option<DateTime<Utc>>,
    pub latest_position_timestamp: Option<DateTime<Utc>>,
    pub engine_power: i32,
    pub engine_building_year: i32,
    pub auxiliary_engine_power: Option<i32>,
    pub auxiliary_engine_building_year: Option<i32>,
    pub boiler_engine_power: Option<i32>,
    pub boiler_engine_building_year: Option<i32>,
    pub engine_type: Option<EngineType>,
    pub engine_rpm: Option<i32>,
    pub service_speed: Option<f64>,
    pub degree_of_electrification: Option<f64>,
}

impl LiveFuelVessel {
    pub fn engines(&self) -> Vec<VesselEngine> {
        engines(
            Some(self.engine_power as f64),
            Some(self.engine_building_year as u32),
            self.auxiliary_engine_power.map(|v| v as f64),
            self.auxiliary_engine_building_year.map(|v| v as u32),
            self.boiler_engine_power.map(|v| v as f64),
            self.boiler_engine_building_year.map(|v| v as u32),
            self.engine_type,
            self.engine_rpm.map(|v| v as u32),
        )
    }
}

#[derive(Debug, Clone)]
pub struct NewVesselConflict {
    pub vessel_id: FiskeridirVesselId,
    pub call_sign: Option<CallSign>,
    pub mmsi: Option<Mmsi>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ActiveVesselConflict {
    pub vessel_ids: Vec<Option<FiskeridirVesselId>>,
    pub call_sign: CallSign,
    pub mmsis: Vec<Option<Mmsi>>,
    pub sources: Vec<Option<VesselSource>>,
}

#[derive(Debug, Clone)]
pub struct Vessel {
    pub fiskeridir: FiskeridirVessel,
    pub ais: Option<AisVessel>,
    pub preferred_trip_assembler: TripAssemblerId,
    pub gear_groups: Vec<GearGroup>,
    pub species_groups: Vec<SpeciesGroup>,
    pub current_trip: Option<VesselCurrentTrip>,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct VesselEvent {
    pub event_id: u64,
    pub vessel_id: FiskeridirVesselId,
    pub report_timestamp: DateTime<Utc>,
    pub occurence_timestamp: Option<DateTime<Utc>>,
    pub event_type: VesselEventType,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum VesselEventOrdering {
    Timestamp = 1,
    ErsRelevantYearMessageNumber = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VesselEventData {
    ErsDep {
        port_id: Option<String>,
        estimated_timestamp: DateTime<Utc>,
    },
    ErsPor {
        port_id: Option<String>,
        estimated_timestamp: DateTime<Utc>,
    },
    Landing,
    ErsDca,
    ErsTra,
    Haul,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VesselEventDetailed {
    pub event_id: u64,
    pub vessel_id: FiskeridirVesselId,
    pub timestamp: DateTime<Utc>,
    pub event_type: VesselEventType,
    pub event_data: VesselEventData,
}

#[derive(Debug, Clone)]
pub struct DepartureWeight {
    pub departure_timestamp: DateTime<Utc>,
    pub weight: f64,
}

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
#[repr(i32)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Hash,
    Ord,
    PartialOrd,
    Serialize_repr,
    Deserialize_repr,
    strum::Display,
    AsRefStr,
    EnumString,
)]
pub enum VesselEventType {
    Landing = 1,
    ErsDca = 2,
    ErsPor = 3,
    ErsDep = 4,
    ErsTra = 5,
    Haul = 6,
}

impl From<VesselEventType> for i32 {
    fn from(value: VesselEventType) -> Self {
        value as i32
    }
}

#[derive(Debug, Clone)]
pub struct FiskeridirVessel {
    pub id: FiskeridirVesselId,
    pub length_group_id: VesselLengthGroup,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owners: Vec<RegisterVesselOwner>,
    pub engine_building_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub building_year: Option<u32>,
    pub auxiliary_engine_power: Option<u32>,
    pub auxiliary_engine_building_year: Option<u32>,
    pub boiler_engine_power: Option<u32>,
    pub boiler_engine_building_year: Option<u32>,
    pub engine_type: Option<EngineType>,
    pub engine_rpm: Option<u32>,
    pub engine_version: u32,
    pub degree_of_electrification: Option<f64>,
    pub service_speed: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
pub fn engines(
    engine_power: Option<f64>,
    engine_building_year: Option<u32>,
    aux_engine_power: Option<f64>,
    aux_engine_building_year: Option<u32>,
    boil_engine_power: Option<f64>,
    boil_engine_building_year: Option<u32>,
    engine_type: Option<EngineType>,
    engine_rpm: Option<u32>,
) -> Vec<VesselEngine> {
    use EngineVariant::*;

    let mut engines = Vec::with_capacity(3);

    let mut f = |p, b, v| {
        if let (Some(p), Some(b)) = (p, b) {
            engines.push(VesselEngine {
                power_kw: p * HP_TO_KW,
                sfc: sfc(b, engine_type, engine_rpm),
                variant: v,
            });
        }
    };

    f(engine_power, engine_building_year, Main);
    f(aux_engine_power, aux_engine_building_year, Auxiliary);
    f(boil_engine_power, boil_engine_building_year, Boiler);

    engines
}

/// Specific Fuel Consumption in `g/kwh` for an engine with the given building year.
pub fn sfc(engine_building_year: u32, engine_type: Option<EngineType>, rpm: Option<u32>) -> f64 {
    // Specific Fuel Consumption `MDO (Marine Diesel Oil)`
    // Source: https://wwwcdn.imo.org/localresources/en/OurWork/Environment/Documents/Fourth%20IMO%20GHG%20Study%202020%20-%20Full%20report%20and%20annexes.pdf
    //         Annex B.2, Table 4
    let sfcs = match engine_building_year {
        ..1984 => [190., 200., 210.],
        1984..2001 => [175., 185., 190.],
        2001.. => [165., 175., 185.],
    };

    if let Some(typ) = engine_type {
        match typ {
            EngineType::SSD => sfcs[0],
            EngineType::MSD => sfcs[1],
            EngineType::HSD => sfcs[2],
        }
    } else if let Some(rpm) = rpm {
        match rpm {
            0..=300 => sfcs[0],
            301..=900 => sfcs[1],
            901.. => sfcs[2],
        }
    } else {
        sfcs.into_iter().mean().unwrap()
    }
}

impl Vessel {
    pub fn id(&self) -> FiskeridirVesselId {
        self.fiskeridir.id
    }
    pub fn fiskeridir_call_sign(&self) -> Option<&CallSign> {
        self.fiskeridir.call_sign.as_ref()
    }
    pub fn mmsi(&self) -> Option<Mmsi> {
        self.ais.as_ref().map(|v| v.mmsi)
    }

    pub fn engines(&self) -> Vec<VesselEngine> {
        engines(
            self.fiskeridir.engine_power.map(|v| v as f64),
            self.fiskeridir.engine_building_year,
            self.fiskeridir.auxiliary_engine_power.map(|v| v as f64),
            self.fiskeridir.auxiliary_engine_building_year,
            self.fiskeridir.boiler_engine_power.map(|v| v as f64),
            self.fiskeridir.boiler_engine_building_year,
            self.fiskeridir.engine_type,
            self.fiskeridir.engine_rpm,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EngineVariant {
    Main,
    Auxiliary,
    Boiler,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, AsRefStr, EnumString)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub enum EngineType {
    /// Slow-Speed Diesel
    SSD = 1,
    /// Medium-Speed Diesel
    MSD = 2,
    /// High-Speed Diesel
    HSD = 3,
}

#[derive(Debug, Clone)]
pub struct VesselEngine {
    pub power_kw: f64,
    pub sfc: f64,
    pub variant: EngineVariant,
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
#[repr(i32)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub enum VesselSource {
    Landings = 1,
    FiskeridirVesselRegister = 2,
}

impl From<VesselSource> for i32 {
    fn from(value: VesselSource) -> Self {
        value as i32
    }
}

impl VesselEventType {
    pub fn name(&self) -> &'static str {
        match self {
            VesselEventType::Landing => "landing",
            VesselEventType::ErsDca => "ers_dca",
            VesselEventType::ErsTra => "ers_tra",
            VesselEventType::ErsDep => "ers_dep",
            VesselEventType::ErsPor => "ers_por",
            VesselEventType::Haul => "haul",
        }
    }
}

#[cfg(feature = "test")]
mod test {
    use super::*;

    impl UpdateVessel {
        pub fn test_new() -> Self {
            Self {
                engine_power: Some(2000),
                engine_building_year: Some(2000),
                auxiliary_engine_power: Some(2000),
                auxiliary_engine_building_year: Some(2000),
                boiler_engine_power: Some(2000),
                boiler_engine_building_year: Some(2000),
                degree_of_electrification: Some(0.5),
                service_speed: Some(15.0),
                engine_type: Some(EngineType::MSD),
                engine_rpm: Some(700),
            }
        }
    }
}
