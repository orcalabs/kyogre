use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use error_stack::{Report, ResultExt};
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    queries::{float_to_decimal, opt_decimal_to_float},
};

#[derive(Debug, Clone)]
pub struct DeliveryPoint {
    pub delivery_point_id: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub latitude: Option<BigDecimal>,
    pub longitude: Option<BigDecimal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, UnnestInsert)]
#[unnest_insert(table_name = "delivery_point_ids", conflict = "delivery_point_id")]
pub struct NewDeliveryPointId {
    pub delivery_point_id: String,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "aqua_culture_register",
    conflict = "delivery_point_id",
    update_all
)]
pub struct AquaCultureEntry {
    pub delivery_point_id: String,
    pub org_id: Option<i32>,
    pub name: String,
    pub address: Option<String>,
    pub zip_code: Option<i32>,
    pub city: Option<String>,
    pub approval_date: NaiveDate,
    pub approval_limit: Option<NaiveDate>,
    pub purpose: String,
    pub production_form: String,
    pub locality_name: String,
    pub locality_municipality_number: i32,
    pub locality_municipality: String,
    pub locality_location: String,
    pub water_environment: String,
    pub locality_kap: BigDecimal,
    pub locality_unit: String,
    pub expiration_date: Option<NaiveDate>,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub prod_omr: Option<String>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "aqua_culture_register_tills",
    conflict = "delivery_point_id,till_nr",
    update_all
)]
pub struct AquaCultureTill {
    pub delivery_point_id: String,
    pub till_nr: String,
    pub till_municipality_number: i32,
    pub till_municipality: String,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "aqua_culture_register_species",
    conflict = "till_nr,till_unit,species_fiskeridir_id",
    update_all
)]
pub struct AquaCultureSpecies {
    pub delivery_point_id: String,
    pub till_nr: String,
    pub till_unit: String,
    pub species_fiskeridir_id: i32,
    pub till_kap: BigDecimal,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "mattilsynet_delivery_points",
    conflict = "delivery_point_id",
    update_all
)]
pub struct MattilsynetDeliveryPoint {
    pub delivery_point_id: String,
    pub name: String,
    pub address: Option<String>,
    pub postal_city: Option<String>,
    pub postal_code: Option<i32>,
}

impl TryFrom<DeliveryPoint> for kyogre_core::DeliveryPoint {
    type Error = Report<PostgresError>;

    fn try_from(v: DeliveryPoint) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v
                .delivery_point_id
                .try_into()
                .change_context(PostgresError::DataConversion)?,
            name: v.name,
            address: v.address,
            latitude: opt_decimal_to_float(v.latitude)
                .change_context(PostgresError::DataConversion)?,
            longitude: opt_decimal_to_float(v.longitude)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}

impl From<fiskeridir_rs::DeliveryPointId> for NewDeliveryPointId {
    fn from(v: fiskeridir_rs::DeliveryPointId) -> Self {
        Self {
            delivery_point_id: v.into_inner(),
        }
    }
}

impl TryFrom<&fiskeridir_rs::AquaCultureEntry> for AquaCultureSpecies {
    type Error = Report<PostgresError>;

    fn try_from(v: &fiskeridir_rs::AquaCultureEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            delivery_point_id: v.delivery_point_id.clone().into_inner(),
            till_nr: v.till_nr.clone(),
            till_unit: v.till_unit.clone(),
            species_fiskeridir_id: v.species_code as i32,
            till_kap: float_to_decimal(v.till_kap).change_context(PostgresError::DataConversion)?,
        })
    }
}

impl TryFrom<&fiskeridir_rs::AquaCultureEntry> for AquaCultureTill {
    type Error = Report<PostgresError>;

    fn try_from(v: &fiskeridir_rs::AquaCultureEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            delivery_point_id: v.delivery_point_id.clone().into_inner(),
            till_nr: v.till_nr.clone(),
            till_municipality_number: v.till_municipality_number as i32,
            till_municipality: v.till_municipality.clone(),
        })
    }
}

impl TryFrom<fiskeridir_rs::AquaCultureEntry> for AquaCultureEntry {
    type Error = Report<PostgresError>;

    fn try_from(v: fiskeridir_rs::AquaCultureEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            delivery_point_id: v.delivery_point_id.into_inner(),
            org_id: v.org_number.map(|o| o as i32),
            name: v.name,
            address: v.address,
            zip_code: v.zip_code.map(|z| z as i32),
            city: v.city,
            approval_date: v.approval_date,
            approval_limit: v.approval_limit,
            purpose: v.purpose,
            production_form: v.production_form,
            locality_name: v.locality_name,
            locality_municipality_number: v.locality_municipality_number as i32,
            locality_municipality: v.locality_municipality,
            locality_location: v.locality_location,
            water_environment: v.water_environment,
            locality_kap: float_to_decimal(v.locality_kap)
                .change_context(PostgresError::DataConversion)?,
            locality_unit: v.locality_unit,
            expiration_date: v.expiration_date,
            latitude: float_to_decimal(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: float_to_decimal(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            prod_omr: v.prod_omr,
        })
    }
}

impl From<kyogre_core::MattilsynetDeliveryPoint> for MattilsynetDeliveryPoint {
    fn from(v: kyogre_core::MattilsynetDeliveryPoint) -> Self {
        Self {
            delivery_point_id: v.id.into_inner(),
            name: v.name,
            address: v.address,
            postal_city: v.postal_city,
            postal_code: v.postal_code.map(|p| p as i32),
        }
    }
}
