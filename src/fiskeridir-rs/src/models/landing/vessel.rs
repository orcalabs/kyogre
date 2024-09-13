use std::{
    fmt::{self, Display},
    num::ParseIntError,
    str::FromStr,
};

use enum_index_derive::{EnumIndex, IndexEnum};
use jurisdiction::Jurisdiction;
use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, Display, EnumCount, EnumIter, EnumString};

use crate::{string_new_types::NonEmptyString, CallSign};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize, FromPrimitive,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct FiskeridirVesselId(i64);

#[derive(Debug, Clone, PartialEq)]
pub struct Vessel {
    pub id: Option<FiskeridirVesselId>,
    pub registration_id: Option<NonEmptyString>,
    pub call_sign: Option<CallSign>,
    pub name: Option<NonEmptyString>,
    pub type_code: Option<VesselType>,
    pub quota_registration_id: Option<NonEmptyString>,
    pub num_crew_members: Option<u32>,
    pub municipality_code: Option<u32>,
    pub municipality_name: Option<NonEmptyString>,
    pub county_code: Option<u32>,
    pub county: Option<NonEmptyString>,
    pub nationality_code: Jurisdiction,
    pub nationality_group: Option<NonEmptyString>,
    pub length: Option<f64>,
    pub length_group_code: Option<VesselLengthGroup>,
    pub length_group_name: Option<NonEmptyString>,
    pub gross_tonnage_1969: Option<u32>,
    pub gross_tonnage_other: Option<u32>,
    pub building_year: Option<u32>,
    pub rebuilding_year: Option<u32>,
    pub engine_power: Option<u32>,
    pub engine_building_year: Option<u32>,
}

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
    EnumIter,
    Serialize_repr,
    Deserialize_repr,
    Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum VesselType {
    Unknown = 0,
    FishingVessel = 1,
    TransportVessel = 2,
    WellBoat = 3,
    CharterVessel = 4,
    PurchaseVessel = 5,
    CoFishingVessel = 6,
    PairTrawlTeam = 7,
    ResearchVessel = 8,
    SchoolVessel = 9,
    BeachSeineVessel = 10,
    KelpTrawler = 11,
    LeisureVessel = 12,
    InvalidFishingVessel = 13,
    SeaweedHarvester = 14,
    WithoutVessel = 15,
}

impl VesselType {
    /// Returns the norwegian name of the vessel type.
    pub fn norwegian_name(&self) -> &'static str {
        use VesselType::*;

        match *self {
            FishingVessel => "Fiskefartøy",
            TransportVessel => "Transportfartøy",
            WellBoat => "Brønnbåt",
            CharterVessel => "Leiefartøy (Erstatningsfartøy)",
            PurchaseVessel => "Kjøpefartøy",
            CoFishingVessel => "Samfiskefartøy",
            PairTrawlTeam => "Partrållag",
            ResearchVessel => "Forskningsfartøy",
            SchoolVessel => "Skolefartøy",
            BeachSeineVessel => "Landnotfartøy",
            KelpTrawler => "Taretråler",
            LeisureVessel => "Fritidsfartøy",
            InvalidFishingVessel => "Ugyldig fiskefartøy",
            SeaweedHarvester => "Tanghøster",
            WithoutVessel => "Uten fartøy",
            Unknown => "Ukjent",
        }
    }
}

impl From<VesselType> for i32 {
    fn from(value: VesselType) -> Self {
        value as i32
    }
}

#[repr(i32)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
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
    EnumIter,
    EnumCount,
    Serialize_repr,
    Deserialize_repr,
    EnumIndex,
    IndexEnum,
    Display,
    AsRefStr,
    EnumString,
)]
pub enum VesselLengthGroup {
    Unknown = 0,
    UnderEleven = 1,
    ElevenToFifteen = 2,
    FifteenToTwentyOne = 3,
    TwentyTwoToTwentyEight = 4,
    TwentyEightAndAbove = 5,
}

impl VesselLengthGroup {
    pub fn description(&self) -> &'static str {
        match self {
            VesselLengthGroup::UnderEleven => "under 11 meter",
            VesselLengthGroup::ElevenToFifteen => "11–14,99 meter",
            VesselLengthGroup::FifteenToTwentyOne => "15–20,99 meter",
            VesselLengthGroup::TwentyTwoToTwentyEight => "21–27,99 meter",
            VesselLengthGroup::TwentyEightAndAbove => "28 m og over",
            VesselLengthGroup::Unknown => "ukjent",
        }
    }
}

impl From<VesselLengthGroup> for i32 {
    fn from(value: VesselLengthGroup) -> Self {
        value as i32
    }
}

impl Vessel {
    pub fn test_default(id: Option<FiskeridirVesselId>, call_sign: &str) -> Vessel {
        Vessel {
            id,
            registration_id: Some("LK-29".parse().unwrap()),
            call_sign: Some(call_sign.parse().unwrap()),
            name: Some("sjarken".parse().unwrap()),
            type_code: Some(VesselType::FishingVessel),
            quota_registration_id: Some("LK-29".parse().unwrap()),
            num_crew_members: Some(10),
            municipality_code: Some(1002),
            municipality_name: Some("Bergen".parse().unwrap()),
            county_code: Some(1230),
            county: Some("Rogaland".parse().unwrap()),
            nationality_code: "NOR".parse().unwrap(),
            nationality_group: Some("Norske fartøy".parse().unwrap()),
            length: Some(16.4),
            length_group_code: Some(VesselLengthGroup::FifteenToTwentyOne),
            length_group_name: Some(
                VesselLengthGroup::FifteenToTwentyOne
                    .description()
                    .parse()
                    .unwrap(),
            ),
            gross_tonnage_1969: Some(143),
            gross_tonnage_other: Some(12),
            building_year: Some(2001),
            rebuilding_year: Some(2010),
            engine_power: Some(900),
            engine_building_year: Some(2000),
        }
    }
}

impl FiskeridirVesselId {
    // This exists because `duckdb-rs` needs to be able to create this type from a
    // generated protobuf schema, and because `DeserializeOptFiskeridirVesselIdStr` needs to
    // construct it from a deserialized i64
    pub fn new(value: i64) -> Self {
        Self(value)
    }
    pub fn into_inner(self) -> i64 {
        self.0
    }
    pub fn test_new(value: i64) -> Self {
        Self(value)
    }
}

impl FromStr for FiskeridirVesselId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl From<FiskeridirVesselId> for i64 {
    fn from(value: FiskeridirVesselId) -> Self {
        value.0
    }
}

impl Display for FiskeridirVesselId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
