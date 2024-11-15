use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, NoneAsEmptyString};

use crate::{deserialize_utils::*, string_new_types::NonEmptyString, CallSign};

use super::FiskeridirVesselId;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub enum RegisterVesselEntityType {
    Company,
    Person,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct RegisterVesselOwner {
    #[serde_as(as = "NoneAsEmptyString")]
    #[cfg_attr(feature = "utoipa", schema(value_type = Option<String>))]
    pub city: Option<NonEmptyString>,
    pub entity_type: RegisterVesselEntityType,
    #[serde(deserialize_with = "opt_from_nullable_str")]
    pub id: Option<i64>,
    #[cfg_attr(feature = "utoipa", schema(value_type = String))]
    pub name: NonEmptyString,
    #[serde_as(as = "PrimitiveFromStr")]
    pub postal_code: i32,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterVessel {
    #[serde(default)]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub engine_power: Option<i32>,
    #[serde_as(as = "DisplayFromStr")]
    pub id: FiskeridirVesselId,
    #[serde(default)]
    #[serde_as(as = "OptPrimitiveFromStr")]
    pub imo_number: Option<i64>,
    pub length: f64,
    #[serde_as(as = "DisplayFromStr")]
    pub municipality_code: i32,
    pub name: NonEmptyString,
    pub owners: Vec<RegisterVesselOwner>,
    pub radio_call_sign: Option<CallSign>,
    pub registration_mark: NonEmptyString,
    #[serde(default)]
    #[serde_as(as = "OptFloatFromStr")]
    pub width: Option<f64>,
}

#[serde_as]
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct RegisterVesselQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_gte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_lte: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub municipality_code: Option<NonEmptyString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub name: Option<NonEmptyString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub query: Option<NonEmptyString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub radio_call_sign: Option<NonEmptyString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub registration_mark: Option<NonEmptyString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_asc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<RegisterVesselSorting>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RegisterVesselSorting {
    Id,
    Name,
    RadioCallSign,
    RegistrationMark,
}

#[cfg(feature = "test")]
mod test {
    use super::*;

    impl RegisterVessel {
        pub fn test_default(id: FiskeridirVesselId) -> Self {
            Self {
                engine_power: Some(200),
                id,
                imo_number: Some(765324),
                length: 16.5,
                municipality_code: 5010,
                name: "Sjarken".parse().unwrap(),
                owners: vec![RegisterVesselOwner::test_default(Some(675432673542))],
                radio_call_sign: Some("LK27".parse().unwrap()),
                registration_mark: "TF3524T".parse().unwrap(),
                width: Some(5.5),
            }
        }
    }

    impl RegisterVesselOwner {
        pub fn test_default(id: Option<i64>) -> Self {
            Self {
                city: Some("TROMSØ".parse().unwrap()),
                entity_type: RegisterVesselEntityType::Person,
                id,
                name: "OWNER A".parse().unwrap(),
                postal_code: 9010,
            }
        }
    }
}
