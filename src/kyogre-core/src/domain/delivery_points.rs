use chrono::NaiveDateTime;
use fiskeridir_rs::{
    BuyerAddress, BuyerLocationId, BuyerLocationType, BuyerPosition, DeliveryPointId, LegalEntityId,
};
use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{buyer_location_error::TooManyApprovalNumbersSnafu, BuyerLocationError};

#[derive(Clone, Debug)]
pub struct DeliveryPoint {
    pub id: DeliveryPointId,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct MattilsynetDeliveryPoint {
    pub id: DeliveryPointId,
    pub name: String,
    pub address: Option<String>,
    pub postal_city: Option<String>,
    pub postal_code: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct ManualDeliveryPoint {
    pub id: DeliveryPointId,
    pub name: String,
    pub type_id: DeliveryPointType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuyerLocation {
    pub id: BuyerLocationId,
    pub delivery_point_id: Option<DeliveryPointId>,
    pub parent: Option<BuyerLocationId>,
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
}

/// Defines different types of delivery points, these values are our own creation and does not
/// originate from a official register.
#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum DeliveryPointType {
    /// We do not know what type of delivery point this is.
    Ukjent = 1,
    /// A regular fish deliery facility.
    Fiskemottak = 2,
    /// Freezing storage.
    Fryselager = 3,
    /// A delivery made to another country than Norway.
    Utland = 4,
    /// Probably a cage (merd).
    AkvakulturLokalitet = 5,
    /// Sold at the docks.
    Kaisalg = 6,
    /// Kelp facility.
    Taremottak = 7,
    /// Facility with hygiene approval to handle fish.
    AnleggGodkjentEtterHygeneKrav = 8,
    /// Exception codes, often related to a specific county.
    MottakerMedUnntak = 9,
    /// A factory.
    Fabrikk = 10,
    /// A factory ship.
    FabrikkSkip = 11,
    /// A sole proprietorship.
    Enkeltmannsforetak = 12,
    /// A third party boat that can be delivered to while at sea.
    Broenbaat = 13,
    /// A vessel with freezing storage.
    FryseSkip = 14,
}

/// The source from where a delivery point id originates from.
#[derive(Debug, Copy, Clone, PartialEq, FromPrimitive, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum DeliveryPointSourceId {
    /// Manual insertion or modification from us.
    Manual = 1,
    /// From Mattilsynet.
    Mattilsynet = 2,
    /// From Fiskeridirektoratets aqua culture register.
    AquaCultureRegister = 3,
    /// From Fiskeridirektoratets buyer register.
    BuyerRegister = 4,
}

impl From<DeliveryPointType> for i32 {
    fn from(value: DeliveryPointType) -> Self {
        value as i32
    }
}

impl From<DeliveryPointSourceId> for i32 {
    fn from(value: DeliveryPointSourceId) -> Self {
        value as i32
    }
}

impl From<MattilsynetDeliveryPoint> for DeliveryPoint {
    fn from(v: MattilsynetDeliveryPoint) -> Self {
        Self {
            id: v.id,
            name: Some(v.name),
            address: v.address,
            latitude: None,
            longitude: None,
        }
    }
}

impl TryFrom<fiskeridir_rs::BuyerLocation> for BuyerLocation {
    type Error = BuyerLocationError;

    fn try_from(v: fiskeridir_rs::BuyerLocation) -> Result<Self, Self::Error> {
        let fiskeridir_rs::BuyerLocation {
            location_id,
            parent,
            location_type,
            legal_entity_id,
            main_legal_entity_id,
            parent_legal_entity_id,
            name,
            created,
            updated,
            address,
            postal_address,
            position,
            approval_numbers,
        } = v;

        let delivery_point_id = match approval_numbers.len() {
            0 | 1 => approval_numbers.into_iter().next(),
            v => return TooManyApprovalNumbersSnafu { num: v }.fail(),
        };

        Ok(Self {
            id: location_id,
            delivery_point_id,
            parent,
            location_type,
            legal_entity_id,
            main_legal_entity_id,
            parent_legal_entity_id,
            name,
            created,
            updated,
            address,
            postal_address,
            position,
        })
    }
}

#[cfg(feature = "test")]
mod test {
    use chrono::Utc;

    use super::*;

    impl MattilsynetDeliveryPoint {
        pub fn test_default() -> Self {
            Self {
                id: DeliveryPointId::new_unchecked("LK17"),
                name: "Name".into(),
                address: Some("Address".into()),
                postal_city: Some("Tromsø".into()),
                postal_code: Some(1234),
            }
        }
    }

    impl BuyerLocation {
        pub fn test_default() -> Self {
            let now = Utc::now().naive_utc();
            Self {
                id: BuyerLocationId::test_default(),
                delivery_point_id: Some(DeliveryPointId::new_unchecked("LK17")),
                parent: None,
                location_type: BuyerLocationType::OrdinaryFacility,
                legal_entity_id: None,
                main_legal_entity_id: None,
                parent_legal_entity_id: None,
                name: Some("Name".into()),
                created: now,
                updated: now,
                address: Some(BuyerAddress::test_default()),
                postal_address: Some(BuyerAddress::test_default()),
                position: Some(BuyerPosition::test_default()),
            }
        }
    }
}
