use num_derive::FromPrimitive;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::Coordinates;

#[derive(Clone, Debug)]
pub struct DeliveryPoint {
    pub coordinates: Option<Coordinates>,
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
    /// From Fiskeridirektoratet main list.
    Fiskeridirektoratet = 1,
    /// From Fiskeridirektoratets aqua culture register.
    AquaCultureRegister = 2,
    /// From Mattilsynet.
    Mattilsynet = 3,
    /// From Sluttseddel/Landingseddel when we have not seen the delivery point id from any other source.
    NoteData = 4,
    /// Manual insertion or modification from us.
    Manual = 5,
}
