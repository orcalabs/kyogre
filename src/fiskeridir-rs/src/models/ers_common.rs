use crate::utils::timestamp_from_date_and_time;
use crate::{deserialize_utils::*, string_new_types::NonEmptyString};
use crate::{SpeciesGroup, SpeciesMainGroup, VesselLengthGroup};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use jurisdiction::Jurisdiction;
use num_derive::FromPrimitive;
use serde::Deserialize;
use serde_repr::Serialize_repr;
use serde_with::serde_as;
use strum::{AsRefStr, Display, EnumString};

use super::{CallSign, FiskeridirVesselId};

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct Port {
    #[serde(rename = "Havn (kode)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub code: Option<NonEmptyString>,
    #[serde(rename = "Havn")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub name: Option<NonEmptyString>,
    #[serde(rename = "Havn nasjonalitet")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub nationality: Option<NonEmptyString>,
}

#[repr(i32)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    FromPrimitive,
    Eq,
    Deserialize,
    Serialize_repr,
    Hash,
    Ord,
    PartialOrd,
    Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "oasgen", derive(oasgen::OaSchema))]
pub enum FiskdirVesselNationalityGroup {
    #[serde(rename(deserialize = "U"))]
    Foreign = 1,
    #[serde(rename(deserialize = "N"))]
    Norwegian = 2,
    #[serde(rename(deserialize = "T"))]
    Test = 3,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsMessageInfo {
    #[serde(rename = "Melding ID")]
    pub message_id: u64,
    #[serde(rename = "Meldingsnummer")]
    pub message_number: u32,
    #[serde(rename = "Meldingsdato")]
    #[serde(deserialize_with = "naive_date_from_str")]
    pub message_date: NaiveDate,
    #[serde(rename = "Meldingsklokkeslett")]
    #[serde(deserialize_with = "naive_time_hour_minutes_from_str")]
    pub message_time: NaiveTime,

    // This field is sometimes missing the `Time` part of the `DateTime`.
    // Therefore, we always use the combination of `message_date` and `message_time` to construct the `DateTime`, and just ignore this field
    #[serde(rename = "Meldingstidspunkt")]
    pub _message_timestamp: String, // DateTime<Utc>

    #[serde(rename = "Meldingstype")]
    pub message_type: NonEmptyString,
    #[serde(rename = "Meldingstype (kode)")]
    pub message_type_code: NonEmptyString,
    #[serde(rename = "Meldingsår")]
    pub message_year: u32,
    #[serde(rename = "Relevant år")]
    pub relevant_year: u32,
    #[serde(rename = "Sekvensnummer")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub sequence_number: Option<u32>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsSpecies {
    #[serde(rename = "Rundvekt")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub living_weight: Option<u32>,
    #[serde(rename = "Art FAO")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub species_fao: Option<NonEmptyString>,
    #[serde(rename = "Art FAO (kode)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub species_fao_code: Option<NonEmptyString>,
    #[serde(rename = "Art - FDIR")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub species_fdir: Option<NonEmptyString>,
    #[serde(rename = "Art - FDIR (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub species_fdir_code: Option<u32>,
    #[serde(rename = "Art - gruppe")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub species_group: Option<NonEmptyString>,
    #[serde(rename = "Art - gruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub species_group_code: Option<SpeciesGroup>,
    #[serde(rename = "Art - hovedgruppe")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub species_main_group: Option<NonEmptyString>,
    #[serde(rename = "Art - hovedgruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub species_main_group_code: Option<SpeciesMainGroup>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsVesselInfo {
    #[serde(rename = "Byggeår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub building_year: Option<u32>,
    #[serde(rename = "Radiokallesignal")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub call_sign: Option<CallSign>,
    #[serde(rename = "Radiokallesignal (ERS)")]
    pub call_sign_ers: CallSign,
    #[serde(rename = "Motorbyggeår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub engine_building_year: Option<u32>,
    #[serde(rename = "Motorkraft")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub engine_power: Option<u32>,
    #[serde(rename = "Bruttotonnasje 1969")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gross_tonnage_1969: Option<u32>,
    #[serde(rename = "Bruttotonnasje annen")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gross_tonnage_other: Option<u32>,
    #[serde(rename = "Fartøyfylke")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub county: Option<NonEmptyString>,
    #[serde(rename = "Fartøyfylke (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub county_code: Option<u32>,
    #[serde(rename = "Største lengde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub greatest_length: Option<f64>,
    #[serde(rename = "Fartøy ID")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub id: Option<FiskeridirVesselId>,
    #[serde(rename = "Fartøyidentifikasjon")]
    pub identification: NonEmptyString,
    #[serde(rename = "Fartøylengde")]
    #[serde_as(as = "FloatFromStr")]
    pub length: f64,
    #[serde(rename = "Lengdegruppe")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub length_group: Option<NonEmptyString>,
    #[serde(rename = "Lengdegruppe (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub length_group_code: Option<VesselLengthGroup>,
    #[serde(rename = "Fartøymateriale (kode)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub material_code: Option<NonEmptyString>,
    #[serde(rename = "Fartøykommune")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub municipality: Option<NonEmptyString>,
    #[serde(rename = "Fartøykommune (kode)")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub municipality_code: Option<u32>,
    #[serde(rename = "Fartøynavn")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub name: Option<NonEmptyString>,
    #[serde(rename = "Fartøynavn (ERS)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub name_ers: Option<NonEmptyString>,
    #[serde(rename = "Fartøynasjonalitet (kode)")]
    #[serde_as(as = "FromStrFromAny")]
    pub nationality_code: Jurisdiction,
    #[serde(rename = "Fartøygruppe (kode)")]
    pub nationality_group_code: FiskdirVesselNationalityGroup,
    #[serde(rename = "Ombyggingsår")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub rebuilding_year: Option<u32>,
    #[serde(rename = "Registreringsmerke")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub registration_id: Option<NonEmptyString>,
    #[serde(rename = "Registreringsmerke (ERS)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub registration_id_ers: Option<NonEmptyString>,
    #[serde(rename = "Fartøy gjelder fra dato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub valid_from: Option<NaiveDate>,
    #[serde(rename = "Fartøy gjelder til dato")]
    #[serde(deserialize_with = "opt_naive_date_from_str")]
    pub valid_until: Option<NaiveDate>,
    #[serde(rename = "Bredde")]
    #[serde_as(as = "OptFloatFromStr")]
    pub width: Option<f64>,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct ErsCatch {
    #[serde(rename = "Kvantum type")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub quantum_type: Option<NonEmptyString>,
    #[serde(rename = "Kvantum type (kode)")]
    #[serde_as(as = "OptFromStrFromAny")]
    pub quantum_type_code: Option<NonEmptyString>,
    #[serde(flatten)]
    pub species: ErsSpecies,
}

impl Port {
    pub fn test_default() -> Self {
        Self {
            code: Some("DENOR".parse().unwrap()),
            name: Some("Nordstrand".parse().unwrap()),
            nationality: Some("NORGE".parse().unwrap()),
        }
    }
}

impl ErsMessageInfo {
    pub fn message_timestamp(&self) -> DateTime<Utc> {
        timestamp_from_date_and_time(self.message_date, self.message_time)
    }

    pub fn set_message_timestamp(&mut self, timestamp: DateTime<Utc>) {
        self.message_time = timestamp.time();
        self.message_date = timestamp.date_naive();
        self.message_year = timestamp.date_naive().year() as u32;
        self.relevant_year = timestamp.date_naive().year() as u32;
    }

    pub fn test_default(message_id: u64, timestamp: DateTime<Utc>, message_number: u32) -> Self {
        Self {
            message_id,
            message_date: timestamp.date_naive(),
            message_number,
            message_time: timestamp.time(),
            _message_timestamp: "".into(),
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
    pub fn test_default(id: Option<FiskeridirVesselId>) -> Self {
        Self {
            id,
            call_sign: Some("LK-23".parse().unwrap()),
            name: Some("Sjarken".parse().unwrap()),
            registration_id: Some("RK-50".parse().unwrap()),
            municipality_code: Some(1232),
            municipality: Some("Oslo".parse().unwrap()),
            county_code: Some(120),
            county: Some("Oslo".parse().unwrap()),
            nationality_code: "NOR".parse().unwrap(),
            length: 50.0,
            length_group: Some(
                VesselLengthGroup::UnderEleven
                    .description()
                    .parse()
                    .unwrap(),
            ),
            length_group_code: Some(VesselLengthGroup::UnderEleven),
            gross_tonnage_1969: Some(5423),
            gross_tonnage_other: Some(4233),
            building_year: Some(2002),
            rebuilding_year: Some(2010),
            engine_power: Some(2332),
            engine_building_year: Some(2000),
            call_sign_ers: "Sjarken".parse().unwrap(),
            greatest_length: Some(50.0),
            identification: "SjarkenID".parse().unwrap(),
            material_code: Some("TRE".parse().unwrap()),
            name_ers: Some("Sjarken".parse().unwrap()),
            nationality_group_code: FiskdirVesselNationalityGroup::Norwegian,
            registration_id_ers: Some("SjarkenID_ERS".parse().unwrap()),
            valid_until: Some((Utc::now() + Duration::days(1000)).date_naive()),
            valid_from: Some((Utc::now() - Duration::days(1000)).date_naive()),
            width: Some(20.0),
        }
    }
}

impl From<FiskdirVesselNationalityGroup> for i32 {
    fn from(value: FiskdirVesselNationalityGroup) -> Self {
        value as i32
    }
}

#[cfg(feature = "test")]
mod test {
    use super::*;

    impl ErsCatch {
        pub fn test_default() -> Self {
            Self {
                quantum_type: Some("Fangst ombord".parse().unwrap()),
                quantum_type_code: Some("OB".parse().unwrap()),
                species: ErsSpecies::test_default(),
            }
        }
    }

    impl ErsSpecies {
        pub fn test_default() -> Self {
            Self {
                living_weight: Some(2468),
                species_fao: Some("Torsk".parse().unwrap()),
                species_fao_code: Some("COD".parse().unwrap()),
                species_fdir: Some("Torsk".parse().unwrap()),
                species_fdir_code: Some(1022),
                species_group: Some(SpeciesGroup::AtlanticCod.norwegian_name().parse().unwrap()),
                species_group_code: Some(SpeciesGroup::AtlanticCod),
                species_main_group: Some(
                    SpeciesMainGroup::CodAndCodishFish
                        .norwegian_name()
                        .parse()
                        .unwrap(),
                ),
                species_main_group_code: Some(SpeciesMainGroup::CodAndCodishFish),
            }
        }
    }
}
