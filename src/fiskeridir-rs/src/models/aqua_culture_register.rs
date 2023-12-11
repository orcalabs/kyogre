use crate::{deserialize_utils::*, DeliveryPointId};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct AquaCultureEntry {
    #[serde(rename = "TILL_NR")]
    pub till_nr: String,
    #[serde(rename = "ORG.NR/PERS.NR")]
    pub org_number: Option<u32>,
    #[serde(rename = "NAVN")]
    pub name: String,
    #[serde(rename = "ADRESSE", deserialize_with = "opt_str_with_hyphen")]
    pub address: Option<String>,
    #[serde(rename = "POSTNR", deserialize_with = "opt_u32_with_hyphen")]
    pub zip_code: Option<u32>,
    #[serde(rename = "POSTSTED", deserialize_with = "opt_str_with_hyphen")]
    pub city: Option<String>,
    #[serde(
        rename = "TILDELINGSTIDSPUNKT",
        deserialize_with = "naive_date_from_str"
    )]
    pub approval_date: NaiveDate,
    #[serde(rename = "TIDSBEGRENSET", deserialize_with = "opt_naive_date_from_str")]
    pub approval_limit: Option<NaiveDate>,
    #[serde(rename = "TILL_KOMNR")]
    pub till_municipality_number: u32,
    #[serde(rename = "TILL_KOM")]
    pub till_municipality: String,
    #[serde(rename = "FORMÅL")]
    pub purpose: String,
    #[serde(rename = "PRODUKSJONSFORM")]
    pub production_form: String,
    #[serde(rename = "ART")]
    pub species: String,
    #[serde(rename = "ART_KODE")]
    pub species_code: u32,
    #[serde(rename = "TILL_KAP")]
    pub till_kap: f64,
    #[serde(rename = "TILL_ENHET")]
    pub till_unit: String,
    #[serde(rename = "LOK_NR")]
    pub delivery_point_id: DeliveryPointId,
    #[serde(rename = "LOK_NAVN")]
    pub locality_name: String,
    #[serde(rename = "LOK_KOMNR")]
    pub locality_municipality_number: u32,
    #[serde(rename = "LOK_KOM")]
    pub locality_municipality: String,
    #[serde(rename = "LOK_PLASS")]
    pub locality_location: String,
    #[serde(rename = "VANNMILJØ")]
    pub water_environment: String,
    #[serde(rename = "LOK_KAP")]
    pub locality_kap: f64,
    #[serde(rename = "LOK_ENHET")]
    pub locality_unit: String,
    #[serde(rename = "UTGÅR_DATO", deserialize_with = "opt_naive_date_from_str")]
    pub expiration_date: Option<NaiveDate>,
    #[serde(rename = "N_GEOWGS84")]
    pub latitude: f64,
    #[serde(rename = "Ø_GEOWGS84")]
    pub longitude: f64,
    #[serde(rename = "PROD_OMR")]
    pub prod_omr: Option<String>,
}

impl AquaCultureEntry {
    pub fn test_default() -> Self {
        let now = Utc::now();

        Self {
            till_nr: "AABB".into(),
            org_number: Some(123),
            name: "AquaCultureEntry".into(),
            address: Some("Address 123".into()),
            zip_code: Some(1234),
            city: Some("Tromso".into()),
            approval_date: now.date_naive(),
            approval_limit: Some(now.date_naive()),
            till_municipality_number: 123,
            till_municipality: "Troms".into(),
            purpose: "Purpose".into(),
            production_form: "Production form".into(),
            species: "Laks".into(),
            species_code: 711,
            till_kap: 1.0,
            till_unit: "LN".into(),
            delivery_point_id: DeliveryPointId::new_unchecked("LK17"),
            locality_name: "Locality".into(),
            locality_municipality_number: 1234,
            locality_municipality: "Troms".into(),
            locality_location: "Location".into(),
            water_environment: "SALTVANN".into(),
            locality_kap: 1.0,
            locality_unit: "TN".into(),
            expiration_date: None,
            latitude: 65.5,
            longitude: 10.1,
            prod_omr: Some("Prod omr".into()),
        }
    }
}
