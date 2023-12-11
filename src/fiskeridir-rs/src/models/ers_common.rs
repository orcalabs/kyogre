use crate::{deserialize_utils::*, string_new_types::NonEmptyString};
use crate::{SpeciesGroup, SpeciesMainGroup, VesselLengthGroup};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::de::{self, Visitor};
use serde::Deserialize;
use serde_repr::Serialize_repr;
use serde_with::{serde_as, NoneAsEmptyString};
use strum_macros::{AsRefStr, EnumString};

#[serde_as]
#[remain::sorted]
#[derive(Deserialize, Debug, Clone)]
pub struct Port {
    #[serde(rename = "Havn (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub code: Option<String>,
    #[serde(rename = "Havn")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub name: Option<String>,
    #[serde(rename = "Havn nasjonalitet")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub nationality: Option<String>,
}

#[repr(i32)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Serialize_repr,
    Hash,
    Ord,
    PartialOrd,
    strum::Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum FiskdirVesselNationalityGroup {
    Foreign = 1,
    Norwegian = 2,
    Test = 3,
}

#[remain::sorted]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsMessageInfo {
    #[serde(rename = "Meldingsdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub message_date: NaiveDate,
    #[serde(rename = "Melding ID")]
    pub message_id: u64,
    #[serde(rename = "Meldingsnummer")]
    pub message_number: u32,
    #[serde(rename = "Meldingsklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub message_time: NaiveTime,
    #[serde(rename = "Meldingstidspunkt")]
    #[serde(deserialize_with = "date_time_utc_from_str")]
    pub message_timestamp: DateTime<Utc>,
    #[serde(rename = "Meldingstype")]
    pub message_type: NonEmptyString,
    #[serde(rename = "Meldingstype (kode)")]
    pub message_type_code: NonEmptyString,
    #[serde(rename = "Meldingsår")]
    pub message_year: u32,
    #[serde(rename = "Relevant år")]
    pub relevant_year: u32,
    #[serde(rename = "Sekvensnummer")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub sequence_number: Option<u32>,
}

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsSpecies {
    #[serde(rename = "Rundvekt")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub living_weight: Option<u32>,
    #[serde(rename = "Art FAO")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub species_fao: Option<String>,
    #[serde(rename = "Art FAO (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub species_fao_code: Option<String>,
    #[serde(rename = "Art - FDIR")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub species_fdir: Option<String>,
    #[serde(rename = "Art - FDIR (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub species_fdir_code: Option<u32>,
    #[serde(rename = "Art - gruppe")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub species_group: Option<String>,
    #[serde(rename = "Art - gruppe (kode)")]
    #[serde(deserialize_with = "species_group_from_opt_value")]
    pub species_group_code: SpeciesGroup,
    #[serde(rename = "Art - hovedgruppe")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub species_main_group: Option<String>,
    #[serde(rename = "Art - hovedgruppe (kode)")]
    #[serde(deserialize_with = "species_main_group_from_opt_value")]
    pub species_main_group_code: SpeciesMainGroup,
}

#[serde_as]
#[remain::sorted]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsVesselInfo {
    #[serde(rename = "Byggeår")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub building_year: Option<u32>,
    #[serde(rename = "Radiokallesignal")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub call_sign: Option<String>,
    #[serde(rename = "Radiokallesignal (ERS)")]
    pub call_sign_ers: NonEmptyString,
    #[serde(rename = "Motorbyggeår")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub engine_building_year: Option<u32>,
    #[serde(rename = "Motorkraft")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub engine_power: Option<u32>,
    #[serde(rename = "Bruttotonnasje 1969")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub gross_tonnage_1969: Option<u32>,
    #[serde(rename = "Bruttotonnasje annen")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub gross_tonnage_other: Option<u32>,
    #[serde(rename = "Fartøyfylke")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_county: Option<String>,
    #[serde(rename = "Fartøyfylke (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_county_code: Option<u32>,
    #[serde(rename = "Største lengde")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub vessel_greatest_length: Option<f64>,
    #[serde(rename = "Fartøy ID")]
    #[serde(deserialize_with = "opt_u64_from_str")]
    pub vessel_id: Option<u64>,
    #[serde(rename = "Fartøyidentifikasjon")]
    pub vessel_identification: NonEmptyString,
    #[serde(rename = "Fartøylengde")]
    #[serde(deserialize_with = "float_from_str")]
    pub vessel_length: f64,
    #[serde(rename = "Lengdegruppe")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_length_group: Option<String>,
    #[serde(rename = "Lengdegruppe (kode)")]
    #[serde(deserialize_with = "opt_enum_from_primitive")]
    pub vessel_length_group_code: Option<VesselLengthGroup>,
    #[serde(rename = "Fartøymateriale (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_material_code: Option<String>,
    #[serde(rename = "Fartøykommune")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_municipality: Option<String>,
    #[serde(rename = "Fartøykommune (kode)")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_municipality_code: Option<u32>,
    #[serde(rename = "Fartøynavn")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_name: Option<String>,
    #[serde(rename = "Fartøynavn (ERS)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_name_ers: Option<String>,
    #[serde(rename = "Fartøynasjonalitet (kode)")]
    pub vessel_nationality_code: NonEmptyString,
    #[serde(rename = "Fartøygruppe (kode)")]
    pub vessel_nationality_group_code: FiskdirVesselNationalityGroup,
    #[serde(rename = "Ombyggingsår")]
    #[serde(deserialize_with = "opt_u32_from_str")]
    pub vessel_rebuilding_year: Option<u32>,
    #[serde(rename = "Registreringsmerke")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub vessel_registration_id: Option<String>,
    #[serde(rename = "Registreringsmerke (ERS)")]
    #[serde(deserialize_with = "opt_string_from_str_or_int")]
    pub vessel_registration_id_ers: Option<String>,
    #[serde(rename = "Fartøy gjelder fra dato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub vessel_valid_from: Option<NaiveDate>,
    #[serde(rename = "Fartøy gjelder til dato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub vessel_valid_until: Option<NaiveDate>,
    #[serde(rename = "Bredde")]
    #[serde(deserialize_with = "opt_float_from_str")]
    pub vessel_width: Option<f64>,
}

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsCatch {
    #[serde(rename = "Kvantum type")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub quantum_type: Option<String>,
    #[serde(rename = "Kvantum type (kode)")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub quantum_type_code: Option<String>,
    #[serde(flatten)]
    pub species: ErsSpecies,
}

impl Port {
    pub fn test_default() -> Self {
        Self {
            code: Some("DENOR".into()),
            name: Some("Nordstrand".into()),
            nationality: Some("NORGE".into()),
        }
    }
}

impl ErsMessageInfo {
    pub fn set_message_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.message_time = timestamp.time();
        self.message_date = timestamp.date_naive();
        self.message_timestamp = timestamp;
        self.message_year = timestamp.date_naive().year() as u32;
        self.relevant_year = timestamp.date_naive().year() as u32;
    }
    pub fn test_default(message_id: u64, timestamp: DateTime<Utc>, message_number: u32) -> Self {
        Self {
            message_id,
            message_date: timestamp.date_naive(),
            message_number,
            message_time: timestamp.time(),
            message_timestamp: timestamp,
            message_type: NonEmptyString::new_unchecked(
                "Detaljert Fangst og aktivitetsmelding".into(),
            ),
            message_type_code: NonEmptyString::new_unchecked("DCA".into()),
            message_year: timestamp.year() as u32,
            relevant_year: timestamp.year() as u32,
            sequence_number: Some(1),
        }
    }
}

impl ErsVesselInfo {
    pub fn test_default(vessel_id: Option<u64>) -> Self {
        Self {
            vessel_id,
            call_sign: Some("LK-23".into()),
            vessel_name: Some("Sjarken".to_owned()),
            vessel_registration_id: Some("RK-50".to_owned()),
            vessel_municipality_code: Some(1232),
            vessel_municipality: Some("Oslo".to_owned()),
            vessel_county_code: Some(120),
            vessel_county: Some("Oslo".to_owned()),
            vessel_nationality_code: NonEmptyString::new_unchecked("NOR".into()),
            vessel_length: 50.0,
            vessel_length_group: Some("21–27,99 meter".into()),
            vessel_length_group_code: Some(VesselLengthGroup::UnderEleven),
            gross_tonnage_1969: Some(5423),
            gross_tonnage_other: Some(4233),
            building_year: Some(2002),
            vessel_rebuilding_year: Some(2010),
            engine_power: Some(2332),
            engine_building_year: Some(2000),
            call_sign_ers: NonEmptyString::new_unchecked("Sjarken".into()),
            vessel_greatest_length: Some(50.0),
            vessel_identification: NonEmptyString::new_unchecked("SjarkenID".into()),
            vessel_material_code: Some("TRE".into()),
            vessel_name_ers: Some("Sjarken".into()),
            vessel_nationality_group_code: FiskdirVesselNationalityGroup::Norwegian,
            vessel_registration_id_ers: Some("SjarkenID_ERS".into()),
            vessel_valid_until: Some((Utc::now() + Duration::days(1000)).date_naive()),
            vessel_valid_from: Some((Utc::now() - Duration::days(1000)).date_naive()),
            vessel_width: Some(20.0),
        }
    }
}

impl ErsCatch {
    pub fn test_default() -> Self {
        Self {
            quantum_type: Some("Fangst ombord".into()),
            quantum_type_code: Some("OB".into()),
            species: ErsSpecies::test_default(),
        }
    }
}

impl ErsSpecies {
    pub fn test_default() -> Self {
        Self {
            living_weight: Some(2468),
            species_fao: Some("Torsk".into()),
            species_fao_code: Some("COD".into()),
            species_fdir: Some("Torsk".into()),
            species_fdir_code: Some(1022),
            species_group: Some(SpeciesGroup::AtlanticCod.norwegian_name().to_owned()),
            species_group_code: SpeciesGroup::AtlanticCod,
            species_main_group: Some(
                SpeciesMainGroup::CodAndCodishFish
                    .norwegian_name()
                    .to_owned(),
            ),
            species_main_group_code: SpeciesMainGroup::CodAndCodishFish,
        }
    }
}

impl From<FiskdirVesselNationalityGroup> for i32 {
    fn from(value: FiskdirVesselNationalityGroup) -> Self {
        value as i32
    }
}

pub struct FiskdirVesselNationalityGroupVisitor;
impl<'de> Visitor<'de> for FiskdirVesselNationalityGroupVisitor {
    type Value = FiskdirVesselNationalityGroup;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a valid FiskdirVesselNationalityGroup")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_i64(v).ok_or(de::Error::invalid_value(de::Unexpected::Signed(v), &self))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Self::Value::from_u64(v).ok_or(de::Error::invalid_value(de::Unexpected::Unsigned(v), &self))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match v {
            "U" => Ok(Self::Value::Foreign),
            "N" => Ok(Self::Value::Norwegian),
            "T" => Ok(Self::Value::Test),
            _ => Err(de::Error::invalid_value(de::Unexpected::Str(v), &self)),
        }
    }
}

impl<'de> Deserialize<'de> for FiskdirVesselNationalityGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(FiskdirVesselNationalityGroupVisitor)
    }
}
