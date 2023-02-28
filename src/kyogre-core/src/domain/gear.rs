use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

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
pub enum Gear {
    UdefinertNot = 10,
    SnurpeNotRingNot = 11,
    LandNot = 12,
    SnurpeNotMedLys = 14,
    LandNotMedLys = 15,
    UdefinertGarn = 20,
    DrivGarn = 21,
    SetteGarn = 22,
    UdefinertKrokredskap = 30,
    Flyteline = 31,
    AndreLiner = 32,
    JuksaPilk = 33,
    DorgHarpSnik = 34,
    Autoline = 35,
    UdefinertBurOgRuser = 40,
    Ruser = 41,
    Teiner = 42,
    Kilenot = 43,
    HavTeiner = 44,
    KrokGarn = 45,
    UdefinertTraal = 50,
    BunnTraal = 51,
    BunnTraalPar = 52,
    FlyteTraal = 53,
    FlyteTraalPar = 54,
    RekeTraal = 55,
    BomTraal = 56,
    KrepseTraal = 57,
    DobbeltTraal = 58,
    TrippelTraal = 59,
    Snurrevad = 61,
    Harpun = 70,
    BrugdeHvalkanon = 71,
    StoerjeHarpun = 72,
    Rifle = 73,
    Annet = 80,
    Skjellskrape = 81,
    Haav = 82,
    TareTraal = 83,
    Tangkutter = 84,
    Haandplukking = 85,
    Oppdrett = 90,
    Uspesifisert = 99,
}

#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Hash,
    Serialize_repr,
    Deserialize_repr,
    PartialOrd,
    Ord,
)]
#[repr(u8)]
pub enum GearGroup {
    Not = 1,
    Garn = 2,
    Krokredskap = 3,
    BurOgRuser = 4,
    Traal = 5,
    Snurrevad = 6,
    HarpunKanon = 7,
    AndreRedskap = 8,
}

#[derive(Debug, Clone, PartialEq, Eq, FromPrimitive, Copy)]
pub enum MainGearGroup {
    Traal = 1,
    Not = 2,
    Konvensjonelle = 3,
    Annet = 4,
}

impl Gear {
    /// Returns the name of the gear type.
    pub fn name(&self) -> &'static str {
        match self {
            Gear::UdefinertNot => "Udefinert not",
            Gear::SnurpeNotRingNot => "Snurpenot/ringnot",
            Gear::LandNot => "Landnot",
            Gear::SnurpeNotMedLys => "Snurepenot med lys",
            Gear::LandNotMedLys => "Landnot med lys",
            Gear::UdefinertGarn => "Udefinert garn",
            Gear::DrivGarn => "Drivgarn",
            Gear::SetteGarn => "Settegarn",
            Gear::UdefinertKrokredskap => "Udefinert krokredskap",
            Gear::Flyteline => "Flyteline",
            Gear::AndreLiner => "Andre liner",
            Gear::JuksaPilk => "Juksa/pilk",
            Gear::DorgHarpSnik => "Dorg/harp/snik",
            Gear::Autoline => "Autoline",
            Gear::UdefinertBurOgRuser => "Udefinert bur og ruser",
            Gear::Ruser => "Ruser",
            Gear::Teiner => "Teiner",
            Gear::Kilenot => "Kilenot",
            Gear::HavTeiner => "Havteiner",
            Gear::KrokGarn => "KrokGarn",
            Gear::UdefinertTraal => "Udefinert trål",
            Gear::BunnTraal => "Bunntrål",
            Gear::BunnTraalPar => "Bunntrål par",
            Gear::FlyteTraal => "Flytetrål",
            Gear::FlyteTraalPar => "Flytetrål par",
            Gear::RekeTraal => "Reketrål",
            Gear::BomTraal => "Bomtrål",
            Gear::KrepseTraal => "Krepsetrål",
            Gear::DobbeltTraal => "Dobbeltrål",
            Gear::TrippelTraal => "Trippeltrål",
            Gear::Snurrevad => "Snurrevad",
            Gear::Harpun => "Harpun og lignende uspesifiserte typer",
            Gear::BrugdeHvalkanon => "Brugde/hvalkanon",
            Gear::StoerjeHarpun => "Størjeharpun",
            Gear::Rifle => "Rifle",
            Gear::Annet => "Annet",
            Gear::Skjellskrape => "Skjelleskrape",
            Gear::Haav => "Håv",
            Gear::TareTraal => "Taretrål",
            Gear::Tangkutter => "Tangkutter",
            Gear::Haandplukking => "Håndplukking",
            Gear::Oppdrett => "Oppdrett",
            Gear::Uspesifisert => "Uspesifisert",
        }
    }

    /// Returns the gear group the gear type is associated with.
    pub fn gear_group(&self) -> GearGroup {
        match self {
            Gear::UdefinertNot
            | Gear::SnurpeNotRingNot
            | Gear::LandNot
            | Gear::SnurpeNotMedLys
            | Gear::LandNotMedLys => GearGroup::Not,
            Gear::UdefinertGarn | Gear::DrivGarn | Gear::SetteGarn => GearGroup::Garn,
            Gear::UdefinertKrokredskap
            | Gear::Flyteline
            | Gear::AndreLiner
            | Gear::JuksaPilk
            | Gear::DorgHarpSnik
            | Gear::Autoline => GearGroup::Krokredskap,
            Gear::UdefinertBurOgRuser
            | Gear::Ruser
            | Gear::Teiner
            | Gear::Kilenot
            | Gear::HavTeiner
            | Gear::KrokGarn => GearGroup::BurOgRuser,
            Gear::UdefinertTraal
            | Gear::BunnTraal
            | Gear::BunnTraalPar
            | Gear::FlyteTraal
            | Gear::FlyteTraalPar
            | Gear::RekeTraal
            | Gear::BomTraal
            | Gear::KrepseTraal
            | Gear::DobbeltTraal
            | Gear::TrippelTraal => GearGroup::Traal,
            Gear::Snurrevad => GearGroup::Snurrevad,
            Gear::Harpun | Gear::BrugdeHvalkanon | Gear::StoerjeHarpun | Gear::Rifle => {
                GearGroup::HarpunKanon
            }
            Gear::Annet
            | Gear::Skjellskrape
            | Gear::Haav
            | Gear::TareTraal
            | Gear::Tangkutter
            | Gear::Haandplukking
            | Gear::Oppdrett
            | Gear::Uspesifisert => GearGroup::AndreRedskap,
        }
    }
}

impl GearGroup {
    /// Returns the name of the gear group type.
    pub fn name(&self) -> &'static str {
        match self {
            GearGroup::Not => "Not",
            GearGroup::Garn => "Garn",
            GearGroup::Krokredskap => "Krokredskap",
            GearGroup::BurOgRuser => "Bur og ruser",
            GearGroup::Traal => "Trål",
            GearGroup::Snurrevad => "Snurrevad",
            GearGroup::HarpunKanon => "Harpun/kanon",
            GearGroup::AndreRedskap => "Andre redskap",
        }
    }
}
