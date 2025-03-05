use fiskeridir_rs::DeliveryPointId;
use serde::{
    Deserialize, Deserializer,
    de::{Error, Unexpected},
};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DeliveryPoint {
    #[serde(rename = "GODKJENNINGSNUMMER")]
    pub id: DeliveryPointId,
    #[serde(rename = "VIRKSOMHETSNAVN", deserialize_with = "string_trim")]
    pub name: String,
    #[serde(rename = "ADRESSE", deserialize_with = "opt_string_trim")]
    pub address: Option<String>,
    #[serde(rename = "POSTNR", deserialize_with = "opt_u32_trim")]
    pub postal_code: Option<u32>,
    #[serde(rename = "POSTSTED", deserialize_with = "opt_string_trim")]
    pub postal_city: Option<String>,

    // Only present for fishery products.
    // Used for filtering out only `Section 8 - Fishery products`.
    #[serde(rename = "SEKSJON", deserialize_with = "opt_string_trim", default)]
    pub section: Option<String>,
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

pub fn opt_string_trim<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(d)?;
    let trimmed = string.trim();

    match trimmed.len() {
        0 => Ok(None),
        v if v == string.len() => Ok(Some(string)),
        _ => Ok(Some(trimmed.into())),
    }
}

pub fn string_trim<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    opt_string_trim(d)?.ok_or_else(|| {
        Error::invalid_value(Unexpected::Other("empty string"), &"a non-empty string")
    })
}

pub fn opt_u32_trim<'de, D>(d: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    match s.trim() {
        "" => Ok(None),
        v => Ok(v.parse().ok()),
    }
}
