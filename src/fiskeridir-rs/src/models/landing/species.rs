use enum_index_derive::{EnumIndex, IndexEnum};
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, EnumCount, EnumIter, EnumString};

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
    strum::Display,
    AsRefStr,
    EnumString,
)]
pub enum SpeciesMainGroup {
    Unknown = 0,
    PelagicFish = 1,
    CodAndCodishFish = 2,
    FlatFishOtherBottomFishAndDeepseaFish = 3,
    ChondrichthyesSharkFishSkatesRaysAndRabbitFish = 4,
    ShellfishMolluscaAndEchinoderm = 5,
    Seaweed = 9,
    FishFarmingFreshWaterFishAndMarineMammals = 99,
}

impl SpeciesMainGroup {
    /// Returns the norwegian name of the species main group type.
    pub fn norwegian_name(&self) -> &'static str {
        match self {
            SpeciesMainGroup::Unknown => "Ukjent",
            SpeciesMainGroup::PelagicFish => "Pelagisk fisk",
            SpeciesMainGroup::CodAndCodishFish => "Torsk og torskeartet fisk",
            SpeciesMainGroup::FlatFishOtherBottomFishAndDeepseaFish => {
                "Flatfisk, annen bunnfisk og dypvannsfisk"
            }
            SpeciesMainGroup::ChondrichthyesSharkFishSkatesRaysAndRabbitFish => {
                "Bruskfisk (haifisk, skater, rokker og havmus)"
            }
            SpeciesMainGroup::ShellfishMolluscaAndEchinoderm => "Skalldyr, bløtdyr og pigghuder",
            SpeciesMainGroup::Seaweed => "Makroalger (tang og tare)",
            SpeciesMainGroup::FishFarmingFreshWaterFishAndMarineMammals => {
                "Oppdrett, ferskvannsfisk og sjøpattedyr"
            }
        }
    }
}

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
    strum::Display,
    AsRefStr,
    EnumString,
)]
pub enum SpeciesGroup {
    Unknown = 0,
    Capelin = 101,
    NorwegianSpringSpawningHerring = 102,
    OtherHerring = 103,
    Mackerel = 104,
    BlueWhiting = 105,
    NorwayPout = 106,
    Sandeels = 107,
    Argentines = 108,
    EuropeanSpratSea = 109,
    EuropeanSpratCoast = 110,
    MesopelagicFish = 111,
    TunaAndTunaishSpecies = 112,
    OtherPelagicFish = 120,
    AtlanticCod = 201,
    Haddock = 202,
    Saithe = 203,
    Gadiformes = 220,
    GreenlandHalibut = 301,
    GoldenRedfish = 302,
    Wrasse = 303,
    Wolffishes = 304,
    FlatFishOtherBottomFishAndDeepseaFish = 320,
    SharkFish = 401,
    SkatesAndOtherChondrichthyes = 402,
    QueenCrab = 501,
    EdibleCrab = 502,
    RedKingCrab = 503,
    RedKingCrabOther = 504,
    NorthernPrawn = 505,
    AntarcticKrill = 506,
    CalanusFinmarchicus = 507,
    OtherShellfishMolluscaAndEchinoderm = 520,
    BrownSeaweed = 901,
    OtherSeaweed = 920,
    FreshWaterFish = 9901,
    FishFarming = 9902,
    MarineMammals = 9903,
    Other = 9920,
}

impl SpeciesGroup {
    /// Returns the norwegian name of the species group type.
    pub fn norwegian_name(&self) -> &'static str {
        match self {
            SpeciesGroup::Unknown => "Ukjent",
            SpeciesGroup::Capelin => "Lodde",
            SpeciesGroup::NorwegianSpringSpawningHerring => "Sild, norsk vårgytende",
            SpeciesGroup::OtherHerring => "Sild, annen",
            SpeciesGroup::Mackerel => "Makrell",
            SpeciesGroup::BlueWhiting => "Kolmule",
            SpeciesGroup::NorwayPout => "Øyepål",
            SpeciesGroup::Sandeels => "Tobis og annen sil",
            SpeciesGroup::Argentines => "Vassild og strømsild",
            SpeciesGroup::EuropeanSpratSea => "Havbrisling",
            SpeciesGroup::EuropeanSpratCoast => "Kystbrisling",
            SpeciesGroup::MesopelagicFish => "Mesopelagisk fisk",
            SpeciesGroup::TunaAndTunaishSpecies => "Tunfisk og tunfisklignende arter",
            SpeciesGroup::OtherPelagicFish => "Annen pelagisk fisk",
            SpeciesGroup::AtlanticCod => "Torsk",
            SpeciesGroup::Haddock => "Hyse",
            SpeciesGroup::Saithe => "Sei",
            SpeciesGroup::Gadiformes => "Annen torskefisk",
            SpeciesGroup::GreenlandHalibut => "Blåkveite",
            SpeciesGroup::GoldenRedfish => "Uer",
            SpeciesGroup::Wrasse => "Leppefisk",
            SpeciesGroup::Wolffishes => "Steinbiter",
            SpeciesGroup::FlatFishOtherBottomFishAndDeepseaFish => {
                "Annen flatfisk, bunnfisk og dypvannsfisk"
            }
            SpeciesGroup::SharkFish => "Haifisk",
            SpeciesGroup::SkatesAndOtherChondrichthyes => "Skater og annen bruskfisk",
            SpeciesGroup::QueenCrab => "Snøkrabbe",
            SpeciesGroup::EdibleCrab => "Taskekrabbe",
            SpeciesGroup::RedKingCrab => "Kongekrabbe, han",
            SpeciesGroup::RedKingCrabOther => "Kongekrabbe, annen",
            SpeciesGroup::NorthernPrawn => "Dypvannsreke",
            SpeciesGroup::AntarcticKrill => "Antarktisk krill",
            SpeciesGroup::CalanusFinmarchicus => "Raudåte",
            SpeciesGroup::OtherShellfishMolluscaAndEchinoderm => {
                "Andre skalldyr, bløtdyr og pigghuder"
            }
            SpeciesGroup::BrownSeaweed => "Brunalger",
            SpeciesGroup::OtherSeaweed => "Andre makroalger",
            SpeciesGroup::FreshWaterFish => "Ferskvannsfisk",
            SpeciesGroup::FishFarming => "Oppdrett",
            SpeciesGroup::MarineMammals => "Sjøpattedyr",
            SpeciesGroup::Other => "Annet",
        }
    }
}

impl From<SpeciesGroup> for i32 {
    fn from(value: SpeciesGroup) -> Self {
        value as i32
    }
}

impl From<SpeciesMainGroup> for i32 {
    fn from(value: SpeciesMainGroup) -> Self {
        value as i32
    }
}
