use std::fmt;

use crate::{AisVessel, Mmsi, TripAssemblerId};
use chrono::{DateTime, Utc};
use fiskeridir_rs::{
    CallSign, FiskeridirVesselId, GearGroup, RegisterVesselOwner, SpeciesGroup, VesselLengthGroup,
};
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

mod benchmark;

pub use benchmark::*;
use strum::{AsRefStr, EnumString};

pub static IGNORED_CONFLICT_CALL_SIGNS: &[&str] = &["00000000", "0"];

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
    pub fish_caught_per_hour: Option<f64>,
    pub gear_groups: Vec<GearGroup>,
    pub species_groups: Vec<SpeciesGroup>,
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

#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RelevantEventType {
    Landing,
    ErsPorAndDep,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize_repr)]
#[repr(i32)]
pub enum VesselBenchmarkId {
    WeightPerHour = 1,
}

impl From<VesselBenchmarkId> for i32 {
    fn from(value: VesselBenchmarkId) -> Self {
        value as i32
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VesselBenchmarkOutput {
    pub vessel_id: FiskeridirVesselId,
    pub benchmark_id: VesselBenchmarkId,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct FiskeridirVessel {
    pub id: FiskeridirVesselId,
    pub vessel_type_id: Option<u32>,
    pub length_group_id: VesselLengthGroup,
    pub nation_group_id: Option<String>,
    pub nation_id: Option<String>,
    pub norwegian_municipality_id: Option<u32>,
    pub norwegian_county_id: Option<u32>,
    pub gross_tonnage_1969: Option<u32>,
    pub gross_tonnage_other: Option<u32>,
    pub call_sign: Option<CallSign>,
    pub name: Option<String>,
    pub registration_id: Option<String>,
    pub length: Option<f64>,
    pub width: Option<f64>,
    pub owner: Option<String>,
    pub owners: Option<Vec<RegisterVesselOwner>>,
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
#[repr(i32)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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

impl std::fmt::Display for VesselBenchmarkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VesselBenchmarkId::WeightPerHour => f.write_str("WeightPerHour"),
        }
    }
}
