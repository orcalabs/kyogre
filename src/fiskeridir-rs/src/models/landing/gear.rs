use enum_index_derive::{EnumIndex, IndexEnum};
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, Display, EnumCount, EnumIter, EnumString};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearDetails {
    pub gear: Gear,
    pub group: GearGroup,
    pub main_group: MainGearGroup,
}

/// Gear code definitions from Fiskedirektoratet.
#[allow(missing_docs)]
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
pub enum Gear {
    Unknown = 0,
    UndefinedSeine = 10,
    PurseSeine = 11,
    BeachSeine = 12,
    PurseSeineWithLight = 14,
    BeachSeineWithLight = 15,
    UndefinedNet = 20,
    DriftNet = 21,
    GillNet = 22,
    UndefinedHookGear = 30,
    FloatingLine = 31,
    OtherLines = 32,
    Jig = 33,
    DorgHarpSnik = 34,
    AutoLine = 35,
    UndefinedLobsterTrapAndFykeNets = 40,
    FykeNets = 41,
    LobsterTraps = 42,
    WedgeSeine = 43,
    OceanLobsterTraps = 44,
    HookNet = 45,
    UndefinedTrawling = 50,
    BottomTrawl = 51,
    BottomTrawlPair = 52,
    MidwaterTrawl = 53,
    MidwaterTrawlPair = 54,
    ShrimpTrawl = 55,
    BeamTrawl = 56,
    CrayfishTrawl = 57,
    DoubleTrawl = 58,
    TripleTrawl = 59,
    DanishSeine = 61,
    Harpoon = 70,
    BaskingSharkWhaleCannon = 71,
    BigHarpoon = 72,
    Rifle = 73,
    Other = 80,
    ShellScrape = 81,
    HandNet = 82,
    KelpTrawl = 83,
    SeaweedCutter = 84,
    HandPicking = 85,
    ShellSucker = 86,
    FishFarming = 90,
    Unspecified = 99,
}

/// GearGroup code definitions from Fiskedirektoratet.
#[allow(missing_docs)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    EnumIter,
    EnumCount,
    EnumIndex,
    IndexEnum,
    Display,
    AsRefStr,
    EnumString,
)]
#[repr(i32)]
pub enum GearGroup {
    /// `Unknown` is added by us as a default value instead of `null`
    Unknown = 0,
    Seine = 1,
    Net = 2,
    HookGear = 3,
    LobsterTrapAndFykeNets = 4,
    Trawl = 5,
    DanishSeine = 6,
    HarpoonCannon = 7,
    OtherGear = 8,
    FishFarming = 9,
}

/// MainGearGroup code definitions from Fiskedirektoratet.
#[allow(missing_docs)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    Debug,
    Clone,
    PartialEq,
    Eq,
    FromPrimitive,
    Copy,
    EnumIter,
    PartialOrd,
    Ord,
    EnumCount,
    EnumIndex,
    IndexEnum,
    Display,
    AsRefStr,
    EnumString,
)]
#[repr(i32)]
pub enum MainGearGroup {
    /// `Unknown` is added by us as a default value instead of `null`
    Unknown = 0,
    Trawl = 1,
    Seine = 2,
    Conventional = 3,
    Other = 4,
}

impl Gear {
    /// Returns the norwegian name of the gear type.
    pub fn norwegian_name(&self) -> &'static str {
        use Gear::*;
        match *self {
            UndefinedSeine => "Udefinert not",
            PurseSeine => "Snurpenot/ringnot",
            BeachSeine => "Landnot",
            PurseSeineWithLight => "Snurepenot med lys",
            BeachSeineWithLight => "Landnot med lys",
            UndefinedNet => "Udefinert garn",
            DriftNet => "Drivgarn",
            GillNet => "Settegarn",
            UndefinedHookGear => "Udefinert krokredskap",
            FloatingLine => "Flyteline",
            OtherLines => "Andre liner",
            Jig => "Juksa/pilk",
            DorgHarpSnik => "Dorg/harp/snik",
            AutoLine => "Autoline",
            UndefinedLobsterTrapAndFykeNets => "Udefinert bur og ruser",
            FykeNets => "Ruser",
            LobsterTraps => "Teiner",
            WedgeSeine => "Kilenot",
            OceanLobsterTraps => "Havteiner",
            HookNet => "KrokGarn",
            UndefinedTrawling => "Udefinert trål",
            BottomTrawl => "Bunntrål",
            BottomTrawlPair => "Bunntrål par",
            MidwaterTrawl => "Flytetrål",
            MidwaterTrawlPair => "Flytetrål par",
            ShrimpTrawl => "Reketrål",
            BeamTrawl => "Bomtrål",
            CrayfishTrawl => "Krepsetrål",
            DoubleTrawl => "Dobbeltrål",
            TripleTrawl => "Trippeltrål",
            DanishSeine => "Snurrevad",
            Harpoon => "Harpun og lignende uspesifiserte typer",
            BaskingSharkWhaleCannon => "Brugde/hvalkanon",
            BigHarpoon => "Størjeharpun",
            Rifle => "Rifle",
            Other => "Annet",
            ShellScrape => "Skjelleskrape",
            HandNet => "Håv",
            KelpTrawl => "Taretrål",
            SeaweedCutter => "Tangkutter",
            HandPicking => "Håndplukking",
            FishFarming => "Oppdrett",
            Unspecified => "Uspesifisert",
            ShellSucker => "Skjellsuger (høstekurv)",
            Unknown => "Ukjent",
        }
    }

    /// Returns the gear group the gear type is associated with.
    pub fn gear_group(&self) -> GearGroup {
        use Gear::*;
        match *self {
            UndefinedSeine | PurseSeine | BeachSeine | PurseSeineWithLight
            | BeachSeineWithLight => GearGroup::Seine,
            UndefinedNet | DriftNet | GillNet => GearGroup::Net,
            UndefinedHookGear | FloatingLine | OtherLines | Jig | DorgHarpSnik | AutoLine => {
                GearGroup::HookGear
            }
            UndefinedLobsterTrapAndFykeNets
            | FykeNets
            | LobsterTraps
            | WedgeSeine
            | OceanLobsterTraps
            | HookNet => GearGroup::LobsterTrapAndFykeNets,
            UndefinedTrawling | BottomTrawl | BottomTrawlPair | MidwaterTrawl
            | MidwaterTrawlPair | ShrimpTrawl | BeamTrawl | CrayfishTrawl | DoubleTrawl
            | TripleTrawl => GearGroup::Trawl,
            DanishSeine => GearGroup::DanishSeine,
            Harpoon | BaskingSharkWhaleCannon | BigHarpoon | Rifle => GearGroup::HarpoonCannon,
            Other | ShellScrape | ShellSucker | HandNet | KelpTrawl | SeaweedCutter
            | HandPicking | FishFarming | Unspecified => GearGroup::OtherGear,
            Unknown => GearGroup::Unknown,
        }
    }
}

impl GearGroup {
    /// Returns the norwegian name of the gear group type.
    pub fn norwegian_name(&self) -> &'static str {
        use GearGroup::*;
        match *self {
            Unknown => "Ukjent",
            Seine => "Not",
            Net => "Garn",
            HookGear => "Krokredskap",
            LobsterTrapAndFykeNets => "Bur og ruser",
            Trawl => "Trål",
            DanishSeine => "Snurrevad",
            HarpoonCannon => "Harpun/kanon",
            OtherGear => "Andre redskap",
            FishFarming => "Oppdrett/uspesifisert",
        }
    }
}

impl MainGearGroup {
    /// Returns the norwegian name of the main gear group type.
    pub fn norwegian_name(&self) -> &'static str {
        use MainGearGroup::*;
        match *self {
            Trawl => "Trål",
            Seine => "Not",
            Conventional => "Konvensjonelle",
            Other => "Annet",
            Unknown => "Ukjent",
        }
    }
}

impl From<Gear> for i32 {
    fn from(value: Gear) -> Self {
        value as i32
    }
}

impl From<GearGroup> for i32 {
    fn from(value: GearGroup) -> Self {
        value as i32
    }
}

impl From<MainGearGroup> for i32 {
    fn from(value: MainGearGroup) -> Self {
        value as i32
    }
}
