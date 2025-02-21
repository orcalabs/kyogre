use fiskeridir_rs::DeliveryPointId;
use serde::Deserialize;
use serde_with::{NoneAsEmptyString, serde_as};

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DeliveryPoint {
    #[serde(rename = "Approval number/ Godkjenningsnummer")]
    pub id: DeliveryPointId,
    #[serde(rename = "Factory name")]
    pub name: String,
    #[serde(rename = "Address")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub address: Option<String>,
    #[serde(rename = "Postal Code / Postnr")]
    pub postal_code: Option<u32>,
    #[serde(rename = "City/ Poststed")]
    #[serde_as(as = "NoneAsEmptyString")]
    pub postal_city: Option<String>,
}

impl From<DeliveryPoint> for kyogre_core::MattilsynetDeliveryPoint {
    fn from(v: DeliveryPoint) -> Self {
        Self {
            id: v.id,
            name: v.name,
            address: v.address,
            postal_city: v.postal_city,
            postal_code: v.postal_code,
        }
    }
}
