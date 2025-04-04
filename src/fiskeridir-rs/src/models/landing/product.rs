use num_derive::{FromPrimitive, ToPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{AsRefStr, Display, EnumString};

use crate::{SpeciesGroup, SpeciesMainGroup, string_new_types::NonEmptyString};

#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    pub condition: Condition,
    pub conservation_method: ConservationMethod,
    pub conservation_method_name: NonEmptyString,
    pub landing_method: Option<LandingMethod>,
    pub landing_method_name: Option<NonEmptyString>,
    pub size_grouping_code: NonEmptyString,
    pub num_fish: Option<u32>,
    pub gross_weight: Option<f64>,
    pub product_weight: f64,
    pub product_weight_over_quota: Option<f64>,
    pub living_weight_over_quota: Option<f64>,
    pub living_weight: Option<f64>,
    pub quality: Quality,
    pub quality_name: NonEmptyString,
    pub purpose: Purpose,
    pub species: Species,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purpose {
    pub code: Option<u32>,
    pub name: Option<NonEmptyString>,
    pub group_code: Option<u32>,
    pub group_name: Option<NonEmptyString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Species {
    pub code: u32,
    pub name: NonEmptyString,
    pub fao_code: Option<NonEmptyString>,
    pub fao_name: Option<NonEmptyString>,
    pub fdir_code: u32,
    pub fdir_name: NonEmptyString,
    pub group_code: SpeciesGroup,
    pub group_name: NonEmptyString,
    pub main_group_code: SpeciesMainGroup,
    pub main_group: NonEmptyString,
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
    Serialize_repr,
    Deserialize_repr,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum LandingMethod {
    ILas = 1,
    Bulk = 2,
    TankBat = 3,
    KasserTonner = 4,
    Bronnbat = 5,
    Kvase = 6,
    Konsumpakket = 7,
    Emballert = 8,
    IMerdOppdrett = 9,
    IMerdOppforet = 10,
    TankBil = 11,
    Kar = 12,
    Container = 13,
    Oppsamlingsfartoy = 14,
    FraMerdUtenForing = 15,
    HalvBlokkKartong25Kg = 71,
    HelBlokkKartong50Kg = 72,
    HalvBlokkSekk25Kg = 73,
    HelBlokkSekk50Kg = 74,
    HalvBlokkUemb50Kg = 75,
    HelBlokkUemb50Kg = 76,
    PbxUemb = 77,
    HelBlokkUemb75Kg = 78,
    HelBlokkKartong75Kg = 79,
    HelBlokkSekk75Kg = 80,
    EmbKartong1Kg = 81,
    EmbKartong5Kg = 82,
    EmbKartong2Kg = 83,
    EmbKartong12Kg = 84,
    Uspesifisert = 99,
}

impl LandingMethod {
    pub fn name(&self) -> &'static str {
        use LandingMethod::*;

        match *self {
            ILas => "i lås",
            Bulk => "bulk",
            TankBat => "tank / båt",
            KasserTonner => "kasser/tønner",
            Bronnbat => "brønnbåt",
            Kvase => "kvase",
            Konsumpakket => "konsumpakket",
            Emballert => "emballert",
            IMerdOppdrett => "i merd, oppdrett fra yngel",
            IMerdOppforet => "i merd, oppforet",
            TankBil => "tank / bil",
            Kar => "kar",
            Container => "container",
            Oppsamlingsfartoy => "Oppsamlingsfartøy",
            FraMerdUtenForing => "Fra merd, uten foring",
            HalvBlokkKartong25Kg => "1/2 blk i kart 25 kg",
            HelBlokkKartong50Kg => "1/1 blk i kart 50 kg",
            HalvBlokkSekk25Kg => "1/2 blk i sekk 25 kg",
            HelBlokkSekk50Kg => "1/1 blk i sekk 50 kg",
            HalvBlokkUemb50Kg => "1/2 blk uemb 25 kg",
            HelBlokkUemb50Kg => "1/1 blk uemb 50 kg",
            PbxUemb => "pbx uemb",
            HelBlokkUemb75Kg => "1/1 blk uemb 75 kg",
            HelBlokkKartong75Kg => "1/1 blk i kart 75 kg",
            HelBlokkSekk75Kg => "1/1 blk i sekk 75 kg",
            EmbKartong1Kg => "1 kg emb kartong",
            EmbKartong5Kg => "5 kg emb kartong",
            EmbKartong2Kg => "2 kg emb kartong",
            EmbKartong12Kg => "12 kg emb kartong",
            Uspesifisert => "Uspesifisert",
        }
    }
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
    Serialize_repr,
    Deserialize_repr,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum ConservationMethod {
    Ensilert = 1,
    FerskOgUkonservert = 2,
    FerskSaltkokt = 3,
    FerskSjokokt = 4,
    Frossen = 5,
    FrossenSaltkokt = 6,
    FrossenSjokokt = 7,
    Gravet = 8,
    Iset = 9,
    Rfw = 10,
    Rsw = 11,
    Rokt = 12,
    Saltet = 13,
    SaltetOgTorket = 14,
    Speket = 15,
    Sukkersaltet = 16,
    Torket = 17,
    RswIs = 18,
    RswOzon = 19,
    RfwOzon = 20,
    RfwIs = 21,
    RfwSyre = 22,
    RfwSyreIs = 23,
    RfwSyreOzon = 24,
    Sws = 25,
    RfwFishForm = 26,
    RfwSoftEddik = 27,
    RswSoftEddik = 28,
    RswFishForm = 29,
    Inndampet = 30,
    Konsentrert = 31,
    Hermetisert = 32,
    FrystOgGlasert = 33,
    AntioksidantbehandletOgFryst = 34,
    Uspesifiert = 99,
}

impl ConservationMethod {
    pub fn name(&self) -> &'static str {
        use ConservationMethod::*;

        match *self {
            Ensilert => "Ensilert",
            FerskOgUkonservert => "Fersk/ukonservert",
            FerskSaltkokt => "Fersk saltkokt",
            FerskSjokokt => "Fersk sjøkokt",
            Frossen => "Frossen",
            FrossenSaltkokt => "Frossen saltkokt",
            FrossenSjokokt => "Frossen sjøkokt",
            Gravet => "Gravet",
            Iset => "Iset",
            Rfw => "Rfw (refrigerated fresh water)",
            Rsw => "Rsw (refrigerated sea water)",
            Rokt => "Røkt",
            Saltet => "Saltet",
            SaltetOgTorket => "Saltet og tørket (klippfisk)",
            Speket => "Speket",
            Sukkersaltet => "Sukkersaltet",
            Torket => "Tørket",
            RswIs => "Rsw + is",
            RswOzon => "Rsw + ozon",
            RfwOzon => "Rfw + ozon",
            RfwIs => "Rfw + is",
            RfwSyre => "Rfw + syre",
            RfwSyreIs => "Rfw + syre + is",
            RfwSyreOzon => "Rfw + syre + ozon",
            Sws => "Sws",
            RfwFishForm => "Rfw + FishForm",
            RfwSoftEddik => "Rfw + \"Soft Eddik\"",
            RswSoftEddik => "Rsw + \"Soft Eddik\"",
            RswFishForm => "Rsw + FishForm",
            Inndampet => "Inndampet",
            Konsentrert => "Konsentrert",
            Hermetisert => "Hermetisert",
            FrystOgGlasert => "Fryst og glasert",
            AntioksidantbehandletOgFryst => "Antioksidantbehandlet og fryst",
            Uspesifiert => "Uspesifisert",
        }
    }
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
    Serialize_repr,
    Deserialize_repr,
    Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub enum Quality {
    Extra = 10,
    Prima = 11,
    Superior = 12,
    A = 20,
    Blank = 21,
    B = 30,
    Sekunda = 31,
    Africa = 32,
    FrostDamagedFish = 33,
    Yellow = 34,
    ProductionRoe = 35,
    CrackedCrab = 36,
    WetCrab = 37,
    WrongCut = 38,
    Injured = 40,
    Offal = 41,
    Wrek = 42,
    Unspecified = 99,
}

impl Quality {
    /// Returns the norwegian name of the quality type.
    pub fn norwegian_name(&self) -> &'static str {
        use Quality::*;

        match *self {
            Extra => "Ekstra",
            Prima => "Prima",
            Superior => "Superior",
            A => "A",
            Blank => "Blank",
            B => "B",
            Sekunda => "Sekunda",
            Africa => "Afrika",
            FrostDamagedFish => "Frostskadet fos",
            Yellow => "Gul",
            ProductionRoe => "Produksjonsrogn",
            CrackedCrab => "Knekt krabbe",
            WetCrab => "Blaut krabbe",
            WrongCut => "Feilkutt",
            Injured => "Skadd",
            Offal => "Offal",
            Wrek => "Vrak",
            Unspecified => "Uspesifisert",
        }
    }
}

/// Produkttilstand
#[repr(i32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    FromPrimitive,
    ToPrimitive,
    Deserialize_repr,
    Serialize_repr,
    Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum Condition {
    Levende = 100,
    Rund = 110,
    Hodekappet = 111,
    RundMedRogn = 112,
    VatTilstand = 115,
    SloydMedHode = 210,
    SloydUtenHodeRundtsnitt = 211,
    SloydUtenHodeUtenOrebein = 212,
    SloydUtenHodeUtenSpord = 213,
    SloydUtenHodeRettsnitt = 214,
    SloydUtenHodeUtenBuk = 215,
    SloydMedHodeUtenSpord = 216,
    SloydMedHodeUtenGjellelokkUtenBrystfinner = 217,
    SloydMedHodeUtenGjeller = 218,
    SloydUtenHodeUtenOrebeinUtenSpordOppdelt = 219,
    NorskkappetUtenSpord = 310,
    Bukskaret = 311,
    Kjakeskaret = 312,
    BukskaretUtenSpord = 313,
    Skivet = 320,
    Pillet = 340,
    Skjellmuskel = 350,
    SkjellmuskelMedRogn = 351,
    Innmat = 352,
    SkadetUtenKlo = 355,
    Skinnet = 360,
    RyggMedSkinn = 361,
    RyggUtenSkinn = 362,
    Ryggbein = 363,
    Rotskjaer = 410,
    Splitt = 411,
    Flekt = 412,
    FiletMedSkinnMedBein = 510,
    FiletUtenSkinnMedBein = 511,
    FiletUtenSkinnUtenBein = 512,
    FiletMedSkinnUtenBein = 513,
    FiletUtenSkinnUtenBeinUtenBuklapp = 514,
    FiletMedSkinnUtenBeinUtenBuklapp = 515,
    LoinsUtenSkinn = 517,
    FiletUtenSkinnUtenBeinWaterJetCutter = 518,
    FiletMedSkinnUtenBeinWaterJetCutter = 519,
    YougumBlokk = 520,
    MixedBlokk = 521,
    FiletATrim = 530,
    FiletBTrim = 531,
    FiletCTrim = 532,
    Akkararmer = 610,
    Belling = 611,
    Finner = 620,
    Buklapper = 621,
    Vinger = 622,
    Haler = 623,
    Klor = 624,
    Skinn = 625,
    Spord = 626,
    Hoder = 630,
    Tunger = 631,
    Kjaker = 632,
    HodeMedBuk = 633,
    KinnOgNakker = 634,
    HodeMedOrebein = 635,
    NakkerUtenKinn = 636,
    Rogn = 641,
    Lever = 642,
    IselMelke = 643,
    Spekk = 644,
    Kjott = 645,
    Svartspekk = 646,
    Hvitspekk = 647,
    Fiskemager = 650,
    SkinnMedSpekk = 651,
    Luffer = 652,
    Penis = 653,
    Ribber = 654,
    Hjerter = 655,
    Farse = 700,
    Surimifarse = 701,
    FarseAvskjaer = 702,
    FarseHelFilet = 703,
    Hydrolysat = 704,
    KrillSmakskonsentrat = 705,
    KrillkjottPellet = 706,
    Krillpulver = 707,
    KrillGranulat = 708,
    Proteinkonsentrat = 709,
    Mel = 710,
    Avskjaer = 800,
    Presset = 810,
    Faks = 811,
    Tran = 820,
    Olje = 830,
    Slo = 900,
    Uspesifisert = 999,
}

impl Condition {
    pub fn name(&self) -> &'static str {
        use Condition::*;

        match *self {
            Levende => "Levende",
            Rund => "Rund",
            Hodekappet => "Hodekappet",
            RundMedRogn => "Rund, m/rogn",
            VatTilstand => "Våt tilstand",
            SloydMedHode => "Sløyd m/hode",
            SloydUtenHodeRundtsnitt => "Sløyd u/hode, rundsnitt ",
            SloydUtenHodeUtenOrebein => "Sløyd u/hode og u/ørebein",
            SloydUtenHodeUtenSpord => "Sløyd u/hode og uten spord",
            SloydUtenHodeRettsnitt => "Sløyd u/hode, rettsnitt ",
            SloydUtenHodeUtenBuk => "Sløyd u/hode, uten buk",
            SloydMedHodeUtenSpord => "Sløyd m/hode og uten spord",
            SloydMedHodeUtenGjellelokkUtenBrystfinner => {
                "Sløyd m/hode, uten gjellelokk, uten brystfinner"
            }
            SloydMedHodeUtenGjeller => "Sløyd med hode uten gjeller",
            SloydUtenHodeUtenOrebeinUtenSpordOppdelt => {
                "Sløyd u/hode, u/ørebein, u/spord, oppdelt i 2-3 stk."
            }
            NorskkappetUtenSpord => "Norskkappet u/spord",
            Bukskaret => "Bukskåret (Japankuttet)",
            Kjakeskaret => "Kjakeskåret",
            BukskaretUtenSpord => "Bukskåret (Japankuttet u/ spord)",
            Skivet => "Skivet",
            Pillet => "Pillet",
            Skjellmuskel => "Skjellmuskel",
            SkjellmuskelMedRogn => "Skjellmuskel m/rogn",
            Innmat => "Innmat",
            SkadetUtenKlo => "Skadet uten klo/gangbein",
            Skinnet => "Skinnet",
            RyggMedSkinn => "Rygg m/skinn",
            RyggUtenSkinn => "Rygg u/skinn",
            Ryggbein => "Ryggbein",
            Rotskjaer => "Rotskjær",
            Splitt => "Splitt",
            Flekt => "Flekt",
            FiletMedSkinnMedBein => "Filet m/skinn og m/bein ",
            FiletUtenSkinnMedBein => "Filet u/skinn, m/bein ",
            FiletUtenSkinnUtenBein => "Filet u/skinn og u/bein",
            FiletMedSkinnUtenBein => "Filet m/skinn, u/bein",
            FiletUtenSkinnUtenBeinUtenBuklapp => "Filet uten skinn, uten bein, uten buklapp",
            FiletMedSkinnUtenBeinUtenBuklapp => "Filet med skinn, uten bein, uten buklapp",
            LoinsUtenSkinn => "Loins uten skinn",
            FiletUtenSkinnUtenBeinWaterJetCutter => {
                "Filet u/skinn og u/bein, water-jet cutter (Valka-kutter)"
            }
            FiletMedSkinnUtenBeinWaterJetCutter => {
                "Filet m/skinn, u/bein, water-jet cutter (Valka-kutter)"
            }
            YougumBlokk => "Yougum blokk",
            MixedBlokk => "Mixed blokk",
            FiletATrim => "Filet, A-trim (maskinelt)",
            FiletBTrim => "Filet, B-trim (maskinelt)",
            FiletCTrim => "Filet, C-trim (maskinelt)",
            Akkararmer => "Akkararmer",
            Belling => "Belling",
            Finner => "Finner",
            Buklapper => "Buklapper",
            Vinger => "Vinger",
            Haler => "Haler",
            Klor => "Klør",
            Skinn => "Skinn",
            Spord => "Spord",
            Hoder => "Hoder",
            Tunger => "Tunger",
            Kjaker => "Kjaker",
            HodeMedBuk => "Hode m/ buk",
            KinnOgNakker => "Kinn og nakker",
            HodeMedOrebein => "Hode m/ørebein",
            NakkerUtenKinn => "Nakker u/kinn",
            Rogn => "Rogn",
            Lever => "Lever",
            IselMelke => "Isel, melke ",
            Spekk => "Spekk",
            Kjott => "Kjøtt",
            Svartspekk => "Svartspekk",
            Hvitspekk => "Hvitspekk",
            Fiskemager => "Fiskemager",
            SkinnMedSpekk => "Skinn m/ spekk",
            Luffer => "Luffer (sveiver)",
            Penis => "Penis",
            Ribber => "Ribber",
            Hjerter => "Hjerter",
            Farse => "Farse",
            Surimifarse => "Surimifarse",
            FarseAvskjaer => "Farse av avskjær",
            FarseHelFilet => "Farse av hel filet",
            Hydrolysat => "Hydrolysat",
            KrillSmakskonsentrat => "Krill Smakskonsentrat",
            KrillkjottPellet => "Krillkjøtt Pellet",
            Krillpulver => "Krillpulver",
            KrillGranulat => "Krill granulat",
            Proteinkonsentrat => "Proteinkonsentrat",
            Mel => "Mel",
            Avskjaer => "Avskjær",
            Presset => "Presset",
            Faks => "Faks",
            Tran => "Tran",
            Olje => "Olje",
            Slo => "Slo",
            Uspesifisert => "Uspesifisert",
        }
    }
}

impl From<Condition> for i32 {
    fn from(value: Condition) -> Self {
        value as i32
    }
}

impl From<Quality> for i32 {
    fn from(value: Quality) -> Self {
        value as i32
    }
}
