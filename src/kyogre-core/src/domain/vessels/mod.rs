use crate::mean::Mean;
use crate::{AisVessel, Mmsi, TripAssemblerId};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    CallSign, FiskeridirVesselId, GearGroup, RegisterVesselOwner, SpeciesGroup, VesselLengthGroup,
};
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, EnumString};

mod benchmark;

pub use benchmark::*;

use super::VesselCurrentTrip;

pub const HP_TO_KW: f64 = 0.745699872;
pub static TEST_SIGNED_IN_VESSEL_CALLSIGN: &str = "LK17";

/// These have been observed in data from Fiskeridirektoratet, and we assume that they are safe to
/// ignore.
pub static IGNORED_CONFLICT_CALL_SIGNS: &[&str] = &["00000000", "0"];

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
    pub degree_of_electrification: Option<f64>,
    pub service_speed: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LiveFuelVessel {
    pub mmsi: Mmsi,
    pub vessel_id: FiskeridirVesselId,
    pub current_trip_start: Option<DateTime<Utc>>,
    pub latest_position_timestamp: Option<DateTime<Utc>>,
    pub engine_building_year: i32,
    pub engine_power: f64,
    pub auxiliary_engine_power: Option<i32>,
    pub auxiliary_engine_building_year: Option<i32>,
    pub boiler_engine_power: Option<i32>,
    pub boiler_engine_building_year: Option<i32>,
    pub service_speed: Option<f64>,
    pub degree_of_electrification: Option<f64>,
}

impl LiveFuelVessel {
    pub fn engines(&self) -> Vec<VesselEngine> {
        let mut vessels = vec![VesselEngine {
            power_kw: self.engine_power * HP_TO_KW,
            sfc: sfc(self.engine_building_year as u32),
            engine_type: EngineType::Main,
        }];

        if let (Some(p), Some(b)) = (
            self.auxiliary_engine_power,
            self.auxiliary_engine_building_year,
        ) {
            vessels.push(VesselEngine {
                power_kw: p as f64 * HP_TO_KW,
                sfc: sfc(b as u32),
                engine_type: EngineType::Auxiliary,
            });
        };

        if let (Some(p), Some(b)) = (self.boiler_engine_power, self.boiler_engine_building_year) {
            vessels.push(VesselEngine {
                power_kw: p as f64 * HP_TO_KW,
                sfc: sfc(b as u32),
                engine_type: EngineType::Boiler,
            });
        };

        vessels
    }
}

#[derive(Debug, Clone)]
pub struct NewVesselConflict {
    pub vessel_id: FiskeridirVesselId,
    pub call_sign: Option<CallSign>,
    pub mmsi: Option<Mmsi>,
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
    pub engine_version: u32,
    pub degree_of_electrification: Option<f64>,
    pub service_speed: Option<f64>,
}

pub fn sfc(engine_building_year: u32) -> f64 {
    // Specific Fuel Consumption
    // Source: https://wwwcdn.imo.org/localresources/en/OurWork/Environment/Documents/Fourth%20IMO%20GHG%20Study%202020%20-%20Full%20report%20and%20annexes.pdf
    //         Annex B.2, Table 4
    match engine_building_year {
        ..1984 => [205., 190., 215., 200., 225., 210.],
        1984..2001 => [185., 175., 195., 185., 205., 190.],
        2001.. => [175., 165., 185., 175., 195., 185.],
    }
    .into_iter()
    .mean()
    .unwrap()
}

impl Vessel {
    pub fn mmsi(&self) -> Option<Mmsi> {
        self.ais.as_ref().map(|v| v.mmsi)
    }

    pub fn engines(&self) -> Vec<VesselEngine> {
        let mut vessels = Vec::with_capacity(3);

        if let (Some(p), Some(b)) = (
            self.fiskeridir.engine_power,
            self.fiskeridir.engine_building_year,
        ) {
            vessels.push(VesselEngine {
                power_kw: p as f64 * HP_TO_KW,
                sfc: sfc(b),
                engine_type: EngineType::Main,
            });
        };

        if let (Some(p), Some(b)) = (
            self.fiskeridir.auxiliary_engine_power,
            self.fiskeridir.auxiliary_engine_building_year,
        ) {
            vessels.push(VesselEngine {
                power_kw: p as f64 * HP_TO_KW,
                sfc: sfc(b),
                engine_type: EngineType::Auxiliary,
            });
        };

        if let (Some(p), Some(b)) = (
            self.fiskeridir.boiler_engine_power,
            self.fiskeridir.boiler_engine_building_year,
        ) {
            vessels.push(VesselEngine {
                power_kw: p as f64 * HP_TO_KW,
                sfc: sfc(b),
                engine_type: EngineType::Boiler,
            });
        };

        vessels
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EngineType {
    Main,
    Auxiliary,
    Boiler,
}
#[derive(Debug, Clone)]
pub struct VesselEngine {
    pub power_kw: f64,
    pub sfc: f64,
    pub engine_type: EngineType,
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
