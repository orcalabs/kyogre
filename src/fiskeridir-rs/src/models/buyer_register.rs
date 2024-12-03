use std::{collections::HashSet, fmt::Display, str::FromStr};

use chrono::NaiveDateTime;
use jurisdiction::Jurisdiction;
use num_derive::FromPrimitive;
use serde::{de::Unexpected, Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{serde_as, DisplayFromStr};
use strum::{AsRefStr, Display, EnumIter, EnumString};

use crate::{sqlx_str_impl, string_new_types::NonEmptyString, ParseStringError};

use super::DeliveryPointId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct BuyerLocationId(i64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct LegalEntityId(NonEmptyString);

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyerLocation {
    pub location_id: BuyerLocationId,
    pub parent: Option<BuyerLocationId>,
    #[serde_as(as = "DisplayFromStr")]
    pub location_type: BuyerLocationType,
    pub legal_entity_id: Option<LegalEntityId>,
    pub main_legal_entity_id: Option<LegalEntityId>,
    pub parent_legal_entity_id: Option<LegalEntityId>,
    pub name: Option<String>,
    pub created: NaiveDateTime,
    pub updated: NaiveDateTime,
    pub address: Option<BuyerAddress>,
    pub postal_address: Option<BuyerAddress>,
    pub position: Option<BuyerPosition>,
    #[serde(deserialize_with = "deserialize_approval_numbers")]
    pub approval_numbers: HashSet<DeliveryPointId>,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyerAddress {
    pub address: Option<String>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub postal_code: Option<u16>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub municipality_number: Option<u16>,
    #[serde_as(as = "DisplayFromStr")]
    pub country_code: Jurisdiction,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuyerPosition {
    pub lat: f64,
    pub lon: f64,
}

#[serde_as]
#[derive(Default, Debug, Clone, Serialize)]
pub struct BuyerLocationsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legal_entity_id: Option<LegalEntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub location_type: Option<BuyerLocationType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_legal_entity_id: Option<LegalEntityId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_legal_entity_id: Option<LegalEntityId>,
    /// Max: 1_000
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_asc: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<BuyerSortField>,
}

#[repr(i32)]
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize_repr,
    FromPrimitive,
    Deserialize_repr,
    EnumIter,
    Display,
    AsRefStr,
    EnumString,
)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
pub enum BuyerLocationType {
    #[strum(serialize = "LEGAL_ENTITY", to_string = "LegalEntity")]
    LegalEntity = 1,
    #[strum(serialize = "DOCK_SALESPERSON_T", to_string = "DockSalesperson")]
    DockSalesperson = 2,
    #[strum(serialize = "TRADER_T", to_string = "Trader")]
    Trader = 3,
    #[strum(serialize = "VESSEL_T", to_string = "Vessel")]
    Vessel = 4,
    #[strum(serialize = "VEHICLE_T", to_string = "Vehicle")]
    Vehicle = 5,
    #[strum(serialize = "ORDINARY_FACILITY_T", to_string = "OrdinaryFacility")]
    OrdinaryFacility = 6,
    #[strum(serialize = "NET_PEN_T", to_string = "NetPen")]
    NetPen = 7,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BuyerSortField {
    LocationId,
    Created,
    Updated,
}

impl Display for BuyerLocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<BuyerLocationId> for i64 {
    fn from(value: BuyerLocationId) -> Self {
        value.0
    }
}

impl From<BuyerLocationType> for i32 {
    fn from(value: BuyerLocationType) -> Self {
        value as i32
    }
}

impl FromStr for LegalEntityId {
    type Err = ParseStringError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl AsRef<str> for LegalEntityId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl LegalEntityId {
    pub fn into_inner(self) -> String {
        self.0.into_inner()
    }
}

sqlx_str_impl!(LegalEntityId);

fn deserialize_approval_numbers<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<HashSet<DeliveryPointId>, D::Error> {
    match Option::<&str>::deserialize(deserializer)? {
        Some(s) => s
            .split(',')
            .map(|v| {
                v.parse().map_err(|_| {
                    serde::de::Error::invalid_value(Unexpected::Str(v), &"a DeliveryPointId")
                })
            })
            .collect(),
        None => Ok(Default::default()),
    }
}

#[cfg(feature = "test")]
mod test {
    use std::sync::atomic::{AtomicI64, Ordering};

    use jurisdiction::Alpha3;

    use super::*;

    impl BuyerLocationId {
        pub fn test_default() -> Self {
            static ID: AtomicI64 = AtomicI64::new(1);
            Self(ID.fetch_add(1, Ordering::Relaxed))
        }
    }

    impl BuyerAddress {
        pub fn test_default() -> Self {
            Self {
                address: Some("Address".into()),
                postal_code: Some(1234),
                municipality_number: Some(4321),
                country_code: Alpha3::NOR.into(),
            }
        }
    }

    impl BuyerPosition {
        pub fn test_default() -> Self {
            Self { lat: 64., lon: 10. }
        }
    }
}
