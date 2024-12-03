use chrono::{NaiveDate, NaiveDateTime};
use fiskeridir_rs::{BuyerAddress, BuyerLocationId, BuyerLocationType, DeliveryPointId};
use kyogre_core::DeliveryPointType;
use unnest_insert::UnnestInsert;

use crate::queries::type_to_i32;

#[derive(Debug, Clone, PartialEq, Eq, Hash, UnnestInsert)]
#[unnest_insert(table_name = "delivery_point_ids", conflict = "delivery_point_id")]
pub struct NewDeliveryPointId<'a> {
    pub delivery_point_id: &'a str,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "manual_delivery_points",
    conflict = "delivery_point_id",
    update_all
)]
pub struct ManualDeliveryPoint {
    pub delivery_point_id: String,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub delivery_point_type_id: DeliveryPointType,
    pub name: String,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "aqua_culture_register",
    conflict = "delivery_point_id",
    update_all
)]
pub struct AquaCultureEntry<'a> {
    pub delivery_point_id: &'a str,
    pub org_id: Option<i32>,
    pub name: &'a str,
    pub address: Option<&'a str>,
    pub zip_code: Option<i32>,
    pub city: Option<&'a str>,
    pub approval_date: NaiveDate,
    pub approval_limit: Option<NaiveDate>,
    pub purpose: &'a str,
    pub production_form: &'a str,
    pub locality_name: &'a str,
    pub locality_municipality_number: i32,
    pub locality_municipality: &'a str,
    pub locality_location: &'a str,
    pub water_environment: &'a str,
    pub locality_kap: f64,
    pub locality_unit: &'a str,
    pub expiration_date: Option<NaiveDate>,
    pub latitude: f64,
    pub longitude: f64,
    pub prod_omr: Option<&'a str>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "aqua_culture_register_tills",
    conflict = "delivery_point_id,till_nr",
    update_all
)]
pub struct AquaCultureTill<'a> {
    pub delivery_point_id: &'a str,
    pub till_nr: &'a str,
    pub till_municipality_number: i32,
    pub till_municipality: &'a str,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "aqua_culture_register_species",
    conflict = "till_nr,till_unit,species_fiskeridir_id",
    update_all
)]
pub struct AquaCultureSpecies<'a> {
    pub delivery_point_id: &'a str,
    pub till_nr: &'a str,
    pub till_unit: &'a str,
    pub species_fiskeridir_id: i32,
    pub till_kap: f64,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "mattilsynet_delivery_points",
    conflict = "delivery_point_id",
    update_all
)]
pub struct MattilsynetDeliveryPoint<'a> {
    pub delivery_point_id: &'a str,
    pub name: &'a str,
    pub address: Option<&'a str>,
    pub postal_city: Option<&'a str>,
    pub postal_code: Option<i32>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "buyer_locations",
    conflict = "buyer_location_id",
    update_all
)]
pub struct NewBuyerLocation<'a> {
    #[unnest_insert(sql_type = "BIGINT", sql_convert = "type_to_i64")]
    pub buyer_location_id: BuyerLocationId,
    #[unnest_insert(sql_type = "TEXT")]
    pub delivery_point_id: Option<&'a DeliveryPointId>,
    #[unnest_insert(sql_type = "BIGINT", sql_convert = "opt_type_to_i64")]
    pub parent: Option<BuyerLocationId>,
    #[unnest_insert(sql_type = "INT", sql_convert = "type_to_i32")]
    pub location_type: BuyerLocationType,
    pub legal_entity_id: Option<&'a str>,
    pub main_legal_entity_id: Option<&'a str>,
    pub parent_legal_entity_id: Option<&'a str>,
    pub name: Option<&'a str>,
    pub created: NaiveDateTime,
    pub updated: NaiveDateTime,
    pub address: Option<&'a str>,
    pub postal_code: Option<i32>,
    pub municipality_number: Option<i32>,
    pub country_code: Option<String>,
    pub postal_address: Option<&'a str>,
    pub postal_postal_code: Option<i32>,
    pub postal_municipality_number: Option<i32>,
    pub postal_country_code: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

impl<'a> From<&'a fiskeridir_rs::DeliveryPointId> for NewDeliveryPointId<'a> {
    fn from(v: &'a fiskeridir_rs::DeliveryPointId) -> Self {
        Self {
            delivery_point_id: v.as_ref(),
        }
    }
}

impl<'a> From<&'a kyogre_core::ManualDeliveryPoint> for NewDeliveryPointId<'a> {
    fn from(v: &'a kyogre_core::ManualDeliveryPoint) -> Self {
        Self {
            delivery_point_id: v.id.as_ref(),
        }
    }
}

impl<'a> From<&'a fiskeridir_rs::AquaCultureEntry> for AquaCultureSpecies<'a> {
    fn from(v: &'a fiskeridir_rs::AquaCultureEntry) -> Self {
        Self {
            delivery_point_id: v.delivery_point_id.as_ref(),
            till_nr: v.till_nr.as_ref(),
            till_unit: v.till_unit.as_ref(),
            species_fiskeridir_id: v.species_code as i32,
            till_kap: v.till_kap,
        }
    }
}

impl<'a> From<&'a fiskeridir_rs::AquaCultureEntry> for AquaCultureTill<'a> {
    fn from(v: &'a fiskeridir_rs::AquaCultureEntry) -> Self {
        Self {
            delivery_point_id: v.delivery_point_id.as_ref(),
            till_nr: v.till_nr.as_ref(),
            till_municipality_number: v.till_municipality_number as i32,
            till_municipality: v.till_municipality.as_ref(),
        }
    }
}

impl<'a> From<&'a fiskeridir_rs::AquaCultureEntry> for AquaCultureEntry<'a> {
    fn from(v: &'a fiskeridir_rs::AquaCultureEntry) -> Self {
        Self {
            delivery_point_id: v.delivery_point_id.as_ref(),
            org_id: v.org_number.map(|o| o as i32),
            name: v.name.as_ref(),
            address: v.address.as_deref(),
            zip_code: v.zip_code.map(|z| z as i32),
            city: v.city.as_deref(),
            approval_date: v.approval_date,
            approval_limit: v.approval_limit,
            purpose: v.purpose.as_ref(),
            production_form: v.production_form.as_ref(),
            locality_name: v.locality_name.as_ref(),
            locality_municipality_number: v.locality_municipality_number as i32,
            locality_municipality: v.locality_municipality.as_ref(),
            locality_location: v.locality_location.as_ref(),
            water_environment: v.water_environment.as_ref(),
            locality_kap: v.locality_kap,
            locality_unit: v.locality_unit.as_ref(),
            expiration_date: v.expiration_date,
            latitude: v.latitude,
            longitude: v.longitude,
            prod_omr: v.prod_omr.as_deref(),
        }
    }
}

impl<'a> From<&'a kyogre_core::MattilsynetDeliveryPoint> for MattilsynetDeliveryPoint<'a> {
    fn from(v: &'a kyogre_core::MattilsynetDeliveryPoint) -> Self {
        Self {
            delivery_point_id: v.id.as_ref(),
            name: v.name.as_ref(),
            address: v.address.as_deref(),
            postal_city: v.postal_city.as_deref(),
            postal_code: v.postal_code.map(|p| p as i32),
        }
    }
}

impl From<kyogre_core::ManualDeliveryPoint> for ManualDeliveryPoint {
    fn from(v: kyogre_core::ManualDeliveryPoint) -> Self {
        Self {
            delivery_point_id: v.id.into_inner(),
            name: v.name,
            delivery_point_type_id: v.type_id,
        }
    }
}

impl<'a> From<&'a kyogre_core::BuyerLocation> for NewBuyerLocation<'a> {
    fn from(v: &'a kyogre_core::BuyerLocation) -> Self {
        let address_expand = |addr: Option<&'a BuyerAddress>| {
            if let Some(v) = addr {
                (
                    v.address.as_deref(),
                    v.postal_code.map(|v| v as i32),
                    v.municipality_number.map(|v| v as i32),
                    Some(v.country_code.alpha3().to_string()),
                )
            } else {
                (None, None, None, None)
            }
        };

        let (address, postal_code, municipality_number, country_code) =
            address_expand(v.address.as_ref());
        let (postal_address, postal_postal_code, postal_municipality_number, postal_country_code) =
            address_expand(v.postal_address.as_ref());

        Self {
            buyer_location_id: v.id,
            delivery_point_id: v.delivery_point_id.as_ref(),
            parent: v.parent,
            location_type: v.location_type,
            legal_entity_id: v.legal_entity_id.as_ref().map(|v| v.as_ref()),
            main_legal_entity_id: v.main_legal_entity_id.as_ref().map(|v| v.as_ref()),
            parent_legal_entity_id: v.parent_legal_entity_id.as_ref().map(|v| v.as_ref()),
            name: v.name.as_deref(),
            created: v.created,
            updated: v.updated,
            address,
            postal_code,
            municipality_number,
            country_code,
            postal_address,
            postal_postal_code,
            postal_municipality_number,
            postal_country_code,
            latitude: v.position.as_ref().map(|v| v.lat),
            longitude: v.position.as_ref().map(|v| v.lon),
        }
    }
}
