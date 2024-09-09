use super::ers_common::{ErsMessageInfo, ErsSpecies, ErsVesselInfo, Port};
use super::FiskeridirVesselId;
use crate::deserialize_utils::*;
use crate::string_new_types::NonEmptyString;
use crate::Gear;
use crate::GearGroup;
use crate::MainGearGroup;
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use num_derive::FromPrimitive;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;
use serde_with::serde_as;
use serde_with::NoneAsEmptyString;
use strum::{AsRefStr, Display, EnumIter, EnumString};

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsDca {
    #[serde(rename = "Aktivitet")]
    pub activity: NonEmptyString,
    #[serde(rename = "Aktivitet (kode)")]
    pub activity_code: NonEmptyString,
    #[serde(rename = "Områdegruppering stopp")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub area_grouping_end: Option<String>,
    #[serde(rename = "Områdegruppering stopp (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub area_grouping_end_code: Option<String>,
    #[serde(rename = "Områdegruppering start")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub area_grouping_start: Option<String>,
    #[serde(rename = "Områdegruppering start (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub area_grouping_start_code: Option<String>,
    #[serde(rename = "Pumpet fra fartøy")]
    pub call_sign_of_loading_vessel: Option<String>,
    #[serde(flatten)]
    pub catch: DcaCatch,
    #[serde(rename = "Fangstår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub catch_year: Option<u32>,
    #[serde(rename = "Varighet")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub duration: Option<u32>,
    #[serde(rename = "Sone")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub economic_zone: Option<String>,
    #[serde(rename = "Sone (kode)")]
    pub economic_zone_code: Option<String>,
    #[serde(flatten)]
    pub gear: GearDca,
    #[serde(rename = "Trekkavstand")]
    pub haul_distance: Option<u32>,
    #[serde(rename = "Sildebestand")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub herring_population: Option<String>,
    #[serde(rename = "Sildebestand (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub herring_population_code: Option<String>,
    #[serde(rename = "Sildebestand - FDIR (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub herring_population_fdir_code: Option<u32>,
    #[serde(rename = "Lokasjon stopp (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub location_end_code: Option<u32>,
    #[serde(rename = "Lokasjon start (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub location_start_code: Option<u32>,
    #[serde(rename = "Hovedområde stopp")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub main_area_end: Option<String>,
    #[serde(rename = "Hovedområde stopp (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub main_area_end_code: Option<u32>,
    #[serde(rename = "Hovedområde start")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub main_area_start: Option<String>,
    #[serde(rename = "Hovedområde start (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub main_area_start_code: Option<u32>,
    #[serde(flatten)]
    pub message_info: ErsMessageInfo,
    #[serde(rename = "Meldingsversjon")]
    pub message_version: u32,
    #[serde(rename = "Havdybde stopp")]
    pub ocean_depth_end: Option<i32>,
    #[serde(rename = "Havdybde start")]
    pub ocean_depth_start: Option<i32>,
    #[serde(flatten)]
    pub port: Port,
    #[serde(rename = "Kvotetype")]
    pub quota_type: NonEmptyString,
    #[serde(rename = "Kvotetype (kode)")]
    pub quota_type_code: u32,
    #[serde(rename = "Startdato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub start_date: Option<NaiveDate>,
    #[serde(rename = "Startposisjon bredde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub start_latitude: Option<f64>,
    #[serde(rename = "Startposisjon lengde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub start_longitude: Option<f64>,
    #[serde(rename = "Startklokkeslett")]
    #[serde(deserialize_with = "opt_naive_time_from_str")]
    pub start_time: Option<NaiveTime>,
    #[serde(rename = "Starttidspunkt")]
    #[serde(deserialize_with = "opt_date_time_utc_from_str")]
    pub start_timestamp: Option<DateTime<Utc>>,
    #[serde(rename = "Stoppdato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub stop_date: Option<NaiveDate>,
    #[serde(rename = "Stopposisjon bredde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub stop_latitude: Option<f64>,
    #[serde(rename = "Stopposisjon lengde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub stop_longitude: Option<f64>,
    #[serde(rename = "Stoppklokkeslett")]
    #[serde(deserialize_with = "opt_naive_time_from_str")]
    pub stop_time: Option<NaiveTime>,
    #[serde(rename = "Stopptidspunkt")]
    #[serde(deserialize_with = "opt_date_time_utc_from_str")]
    pub stop_timestamp: Option<DateTime<Utc>>,
    #[serde(flatten)]
    pub vessel_info: ErsVesselInfo,
    #[serde(flatten)]
    pub whale_catch_info: WhaleCatchInfo,
}

/// It seems that either none or all of these fields are present in DCA messages.
#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct GearDca {
    #[serde(rename = "Redskap mengde")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_amount: Option<u32>,
    #[serde(rename = "Redskap FAO")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gear_fao: Option<String>,
    #[serde(rename = "Redskap FAO (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gear_fao_code: Option<String>,
    #[serde(rename = "Redskap FDIR")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gear_fdir: Option<String>,
    #[serde(rename = "Redskap FDIR (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_fdir_code: Option<Gear>,
    #[serde(rename = "Redskap - gruppe")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gear_group: Option<String>,
    #[serde(rename = "Redskap - gruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_group_code: Option<GearGroup>,
    #[serde(rename = "Redskap - hovedgruppe")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gear_main_group: Option<String>,
    #[serde(rename = "Redskap - hovedgruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_main_group_code: Option<MainGearGroup>,
    #[serde(rename = "Redskap maskevidde")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_mesh_width: Option<u32>,
    #[serde(rename = "Redskap problem")]
    pub gear_problem: Option<String>,
    #[serde(rename = "Redskap problem (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_problem_code: Option<u32>,
    #[serde(rename = "Redskapsspesifikasjon")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gear_specification: Option<String>,
    #[serde(rename = "Redskapsspesifikasjon (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gear_specification_code: Option<u32>,
}

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct DcaCatch {
    #[serde(rename = "Hovedart FAO")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub majority_species_fao: Option<String>,
    #[serde(rename = "Hovedart FAO (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub majority_species_fao_code: Option<String>,
    #[serde(rename = "Hovedart - FDIR (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub majority_species_fdir_code: Option<u32>,
    #[serde(flatten)]
    pub species: ErsSpecies,
}

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct WhaleCatchInfo {
    #[serde(rename = "Spekkmål A")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub blubber_measure_a: Option<u32>,
    #[serde(rename = "Spekkmål B")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub blubber_measure_b: Option<u32>,
    #[serde(rename = "Spekkmål C")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub blubber_measure_c: Option<u32>,
    #[serde(rename = "Omkrets")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub circumference: Option<u32>,
    #[serde(rename = "Fosterlengde")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub fetus_length: Option<u32>,
    #[serde(rename = "Kjønn")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub gender: Option<String>,
    #[serde(rename = "Kjønn (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gender_code: Option<WhaleGender>,
    #[serde(rename = "Granatnummer")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub grenade_number: Option<String>,
    #[serde(rename = "Individnummer")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub individual_number: Option<u32>,
    #[serde(rename = "Lengde")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub length: Option<u32>,
}

#[allow(missing_docs)]
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
pub enum WhaleGender {
    Male = 1,
    Female = 2,
}

impl From<WhaleGender> for i32 {
    fn from(value: WhaleGender) -> Self {
        value as i32
    }
}

impl ErsDca {
    pub fn set_start_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.start_time = Some(timestamp.time());
        self.start_date = Some(timestamp.date_naive());
        self.start_timestamp = Some(timestamp);
    }
    pub fn set_stop_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.stop_time = Some(timestamp.time());
        self.stop_date = Some(timestamp.date_naive());
        self.stop_timestamp = Some(timestamp);
    }
    pub fn test_default(message_id: u64, vessel_id: Option<FiskeridirVesselId>) -> Self {
        let stop = Utc::now();
        let start = stop - Duration::hours(4);
        Self {
            activity: NonEmptyString::new_unchecked("Fiske overført".into()),
            activity_code: NonEmptyString::new_unchecked("FIS".into()),
            area_grouping_end: Some("Atlanterhavet, nordøst/sentrale Nordsjøen".into()),
            area_grouping_end_code: Some("27_4_B".into()),
            area_grouping_start: Some("Atlanterhavet, nordøst/sentrale Nordsjøen".into()),
            area_grouping_start_code: Some("27_4_B".into()),
            call_sign_of_loading_vessel: Some("LK-23".into()),
            catch: DcaCatch::test_default(),
            catch_year: Some(start.year() as u32),
            duration: Some(30),
            economic_zone: Some("Norges økonomiske sone".into()),
            economic_zone_code: Some("NOR".into()),
            gear: GearDca::test_default(),
            haul_distance: Some(1000),
            herring_population: Some("Norsk vårgytende sild".into()),
            herring_population_code: Some("NOR01".into()),
            herring_population_fdir_code: Some(61104),
            location_end_code: Some(30),
            location_start_code: Some(30),
            main_area_end: Some("Sentrale Norskehav".into()),
            main_area_end_code: Some(9),
            main_area_start: Some("Sentrale Norskehav".into()),
            main_area_start_code: Some(9),
            message_info: ErsMessageInfo::test_default(message_id, stop, 1),
            message_version: 1,
            ocean_depth_end: Some(5432),
            ocean_depth_start: Some(3452),
            port: Port::test_default(),
            quota_type: NonEmptyString::new_unchecked("Vanlig kvote".into()),
            quota_type_code: 1,
            start_date: Some(start.date_naive()),
            start_latitude: Some(57.81891926743023),
            start_longitude: Some(7.6702187769988495),
            start_time: Some(start.time()),
            start_timestamp: Some(start),
            stop_date: Some(stop.date_naive()),
            stop_latitude: Some(57.82),
            stop_longitude: Some(7.68),
            stop_time: Some(stop.time()),
            stop_timestamp: Some(stop),
            vessel_info: ErsVesselInfo::test_default(vessel_id),
            whale_catch_info: WhaleCatchInfo::test_default(),
        }
    }
}

impl GearDca {
    pub fn test_default() -> Self {
        Self {
            gear_amount: Some(1),
            gear_fao: Some("Traal".into()),
            gear_fao_code: Some("TR".into()),
            gear_fdir: Some("Trippel Traal".into()),
            gear_fdir_code: Some(Gear::TripleTrawl),
            gear_group: Some("Traal".into()),
            gear_group_code: Some(GearGroup::Trawl),
            gear_main_group: Some("Traal".into()),
            gear_main_group_code: Some(MainGearGroup::Trawl),
            gear_mesh_width: Some(6),
            gear_problem: None,
            gear_problem_code: None,
            gear_specification: Some("enkeltrål".into()),
            gear_specification_code: Some(1),
        }
    }
}

impl DcaCatch {
    pub fn test_default() -> Self {
        Self {
            majority_species_fao: Some("Torsk".into()),
            majority_species_fao_code: Some("COD".into()),
            majority_species_fdir_code: Some(1022),
            species: ErsSpecies::test_default(),
        }
    }
}

impl WhaleCatchInfo {
    pub fn test_default() -> Self {
        Self {
            blubber_measure_a: Some(1),
            blubber_measure_b: Some(2),
            blubber_measure_c: Some(3),
            circumference: Some(4),
            fetus_length: Some(5),
            gender: Some("Hannkjønn".into()),
            gender_code: Some(WhaleGender::Male),
            grenade_number: Some("WhaleGrenade1".into()),
            individual_number: Some(643),
            length: Some(622),
        }
    }
}
