use crate::{deserialize_utils::*, string_new_types::NonEmptyString, CallSign};
use chrono::{DateTime, Utc};
use serde_with::NoneAsEmptyString;

use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[remain::sorted]
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct Vms {
    #[serde(rename = "Radiokallesignal")]
    pub call_sign: CallSign,
    #[serde(rename = "Kurs")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub course: Option<u32>,
    #[serde(rename = "Bruttotonnasje", alias = "Bruttotonnasje 1969")]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub gross_tonnage: Option<u32>,
    #[serde(rename = "Breddegrad")]
    #[serde_as(as = "OptFloatFromStr")]
    pub latitude: Option<f64>,
    #[serde(rename = "Lengdegrad")]
    #[serde_as(as = "OptFloatFromStr")]
    pub longitude: Option<f64>,
    #[serde(rename = "Melding ID", alias = "MeldingID")]
    #[serde_as(as = "DisplayFromStr")]
    pub message_id: u32,
    #[serde(rename = "Meldingstype")]
    pub message_type: NonEmptyString,
    #[serde(rename = "Meldingstype (kode)", alias = "Meldingstype(kode)")]
    pub message_type_code: NonEmptyString,
    #[serde_as(as = "NoneAsEmptyString")]
    #[serde(rename = "Registreringsmerke")]
    pub registration_id: Option<NonEmptyString>,
    #[serde(rename = "Fart")]
    #[serde_as(as = "OptFloatFromStr")]
    pub speed: Option<f64>,
    #[serde(deserialize_with = "date_time_utc_from_non_iso_utc_str")]
    #[serde(rename = "Tidspunkt (UTC)", alias = "Tidspunkt(UTC)")]
    pub timestamp: DateTime<Utc>,
    #[serde(rename = "Største lengde", alias = "Størstelengde")]
    #[serde_as(as = "FloatFromStr")]
    pub vessel_length: f64,
    #[serde(rename = "Fartøynavn")]
    pub vessel_name: NonEmptyString,
    #[serde(rename = "Fartøytype")]
    pub vessel_type: NonEmptyString,
}

impl Vms {
    pub fn test_default(message_id: u32, call_sign: CallSign, timestamp: DateTime<Utc>) -> Vms {
        Vms {
            call_sign,
            course: Some(81),
            gross_tonnage: Some(28),
            latitude: Some(37.123),
            longitude: Some(28.123),
            message_id,
            message_type: "Position".parse().unwrap(),
            message_type_code: "POS".parse().unwrap(),
            registration_id: Some("LK-123".parse().unwrap()),
            speed: Some(8.12),
            timestamp,
            vessel_length: 18.01,
            vessel_name: "sjarken".parse().unwrap(),
            vessel_type: "Fiskefartøy".parse().unwrap(),
        }
    }
}
