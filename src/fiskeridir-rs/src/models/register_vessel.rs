use serde::{Deserialize, Serialize};

use crate::{deserialize_utils::*, CallSign};

#[remain::sorted]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum RegisterVesselEntityType {
    Company,
    Person,
}

#[remain::sorted]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RegisterVesselOwner {
    pub city: Option<String>,
    pub entity_type: RegisterVesselEntityType,
    #[serde(deserialize_with = "opt_i64_from_nullable_str")]
    pub id: Option<i64>,
    pub name: String,
    #[serde(deserialize_with = "i32_from_str")]
    pub postal_code: i32,
}

#[remain::sorted]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterVessel {
    #[serde(deserialize_with = "opt_i32_from_str", default)]
    pub engine_power: Option<i32>,
    #[serde(deserialize_with = "i64_from_str")]
    pub id: i64,
    #[serde(deserialize_with = "opt_i64_from_str", default)]
    pub imo_number: Option<i64>,
    pub length: f64,
    #[serde(deserialize_with = "i32_from_str")]
    pub municipality_code: i32,
    pub name: String,
    pub owners: Vec<RegisterVesselOwner>,
    pub radio_call_sign: Option<CallSign>,
    pub registration_mark: String,
    #[serde(deserialize_with = "opt_float_from_str", default)]
    pub width: Option<f64>,
}

#[remain::sorted]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct RegisterVesselQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_gte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_lte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub municipality_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_call_sign: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_mark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_asc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<RegisterVesselSorting>,
}

#[remain::sorted]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RegisterVesselSorting {
    Id,
    Name,
    RadioCallSign,
    RegistrationMark,
}

impl RegisterVessel {
    pub fn test_default(id: i64) -> Self {
        Self {
            engine_power: Some(200),
            id,
            imo_number: Some(765324),
            length: 16.5,
            municipality_code: 5010,
            name: "Sjarken".into(),
            owners: vec![RegisterVesselOwner::test_default(Some(675432673542))],
            radio_call_sign: Some(CallSign::try_from("LK27").unwrap()),
            registration_mark: "TF3524T".into(),
            width: Some(5.5),
        }
    }
}

impl RegisterVesselOwner {
    pub fn test_default(id: Option<i64>) -> Self {
        Self {
            city: Some("TROMSØ".into()),
            entity_type: RegisterVesselEntityType::Person,
            id,
            name: "OWNER A".into(),
            postal_code: 9010,
        }
    }
}
