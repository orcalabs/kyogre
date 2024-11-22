use crate::{
    error::Error,
    queries::{opt_type_to_i64, type_to_i32},
};
use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::{CallSign, FiskdirVesselNationalityGroup, SpeciesGroup, SpeciesMainGroup};
use kyogre_core::{ErsQuantumType, FiskeridirVesselId};
use serde::Deserialize;
use unnest_insert::UnnestInsert;

#[derive(Debug, Clone)]
pub struct Tra {
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub reloading_timestamp: Option<DateTime<Utc>>,
    pub message_timestamp: DateTime<Utc>,
    pub catches: String,
    pub reload_to: Option<FiskeridirVesselId>,
    pub reload_from: Option<FiskeridirVesselId>,
    pub reload_to_call_sign: Option<CallSign>,
    pub reload_from_call_sign: Option<CallSign>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TripTra {
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub reloading_timestamp: Option<DateTime<Utc>>,
    pub message_timestamp: DateTime<Utc>,
    pub catches: Vec<TripTraCatch>,
    pub reload_to: Option<FiskeridirVesselId>,
    pub reload_from: Option<FiskeridirVesselId>,
    pub reload_to_call_sign: Option<CallSign>,
    pub reload_from_call_sign: Option<CallSign>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TripTraCatch {
    pub living_weight: i32,
    pub species_group_id: SpeciesGroup,
    pub catch_quantum: ErsQuantumType,
}

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "ers_tra",
    returning = "vessel_event_id, message_id, message_timestamp, reloading_timestamp, fiskeridir_vessel_id"
)]
pub struct NewErsTra<'a> {
    pub message_id: i64,
    pub message_number: i32,
    pub message_timestamp: DateTime<Utc>,
    pub ers_message_type_id: &'a str,
    pub message_year: i32,
    pub relevant_year: i32,
    pub sequence_number: Option<i32>,
    pub start_latitude: Option<f64>,
    pub start_longitude: Option<f64>,
    pub reloading_timestamp: Option<DateTime<Utc>>,
    pub reload_to_vessel_call_sign: Option<&'a str>,
    pub reload_from_vessel_call_sign: Option<&'a str>,
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "opt_type_to_i64")]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_building_year: Option<i32>,
    pub vessel_call_sign: Option<&'a str>,
    pub vessel_call_sign_ers: &'a str,
    pub vessel_engine_building_year: Option<i32>,
    pub vessel_engine_power: Option<i32>,
    pub vessel_gross_tonnage_1969: Option<i32>,
    pub vessel_gross_tonnage_other: Option<i32>,
    pub vessel_county: Option<&'a str>,
    pub vessel_county_code: Option<i32>,
    pub vessel_greatest_length: Option<f64>,
    pub vessel_identification: &'a str,
    pub vessel_length: f64,
    pub vessel_length_group: Option<&'a str>,
    pub vessel_length_group_code: Option<i32>,
    pub vessel_material_code: Option<&'a str>,
    pub vessel_municipality: Option<&'a str>,
    pub vessel_municipality_code: Option<i32>,
    pub vessel_name: Option<&'a str>,
    pub vessel_name_ers: Option<&'a str>,
    pub vessel_nationality_code: String,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub fiskeridir_vessel_nationality_group_id: FiskdirVesselNationalityGroup,
    pub vessel_rebuilding_year: Option<i32>,
    pub vessel_registration_id: Option<&'a str>,
    pub vessel_registration_id_ers: Option<&'a str>,
    pub vessel_valid_until: Option<NaiveDate>,
    pub vessel_valid_from: Option<NaiveDate>,
    pub vessel_width: Option<f64>,
    pub vessel_event_id: Option<i64>,
}

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "ers_tra_catches",
    conflict = "message_id,ers_quantum_type_id,species_fao_id"
)]
pub struct NewErsTraCatch<'a> {
    pub message_id: i64,
    pub ers_quantum_type_id: Option<&'a str>,
    pub living_weight: Option<i32>,
    pub species_fao_id: Option<&'a str>,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: i32,
    pub species_main_group_id: i32,
}

impl<'a> From<&'a fiskeridir_rs::ErsTra> for NewErsTra<'a> {
    fn from(v: &'a fiskeridir_rs::ErsTra) -> Self {
        Self {
            message_id: v.message_info.message_id as i64,
            message_number: v.message_info.message_number as i32,
            message_timestamp: v.message_info.message_timestamp(),
            reloading_timestamp: v.reloading_timestamp(),
            ers_message_type_id: v.message_info.message_type_code.as_ref(),
            message_year: v.message_info.message_year as i32,
            relevant_year: v.message_info.relevant_year as i32,
            sequence_number: v.message_info.sequence_number.map(|v| v as i32),
            fiskeridir_vessel_id: v.vessel_info.id,
            vessel_building_year: v.vessel_info.building_year.map(|v| v as i32),
            vessel_call_sign: v.vessel_info.call_sign.as_deref(),
            vessel_call_sign_ers: v.vessel_info.call_sign_ers.as_ref(),
            vessel_engine_building_year: v.vessel_info.engine_building_year.map(|v| v as i32),
            vessel_engine_power: v.vessel_info.engine_power.map(|v| v as i32),
            vessel_gross_tonnage_1969: v.vessel_info.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: v.vessel_info.gross_tonnage_other.map(|v| v as i32),
            vessel_county: v.vessel_info.county.as_deref(),
            vessel_county_code: v.vessel_info.county_code.map(|v| v as i32),
            vessel_greatest_length: v.vessel_info.greatest_length,
            vessel_identification: v.vessel_info.identification.as_ref(),
            vessel_length: v.vessel_info.length,
            vessel_length_group: v.vessel_info.length_group.as_deref(),
            vessel_length_group_code: v.vessel_info.length_group_code.map(|v| v as i32),
            vessel_material_code: v.vessel_info.material_code.as_deref(),
            vessel_municipality: v.vessel_info.municipality.as_deref(),
            vessel_municipality_code: v.vessel_info.municipality_code.map(|v| v as i32),
            vessel_name: v.vessel_info.name.as_deref(),
            vessel_name_ers: v.vessel_info.name_ers.as_deref(),
            vessel_nationality_code: v.vessel_info.nationality_code.alpha3().to_string(),
            fiskeridir_vessel_nationality_group_id: v.vessel_info.nationality_group_code,
            vessel_rebuilding_year: v.vessel_info.rebuilding_year.map(|v| v as i32),
            vessel_registration_id: v.vessel_info.registration_id.as_deref(),
            vessel_registration_id_ers: v.vessel_info.registration_id_ers.as_deref(),
            vessel_valid_until: v.vessel_info.valid_until,
            vessel_valid_from: v.vessel_info.valid_from,
            vessel_width: v.vessel_info.width,
            vessel_event_id: None,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            reload_to_vessel_call_sign: v.reloading_to_vessel.as_deref(),
            reload_from_vessel_call_sign: v.reloading_from_vessel.as_deref(),
        }
    }
}

impl<'a> NewErsTraCatch<'a> {
    pub fn from_ers_tra(ers_tra: &'a fiskeridir_rs::ErsTra) -> Option<Self> {
        let c = &ers_tra.catch;
        let s = &c.species;

        // According to our understanding, all the fields of a `NewErsTraCatch` (except `species_fiskeridir_id`)
        // are required, meaning if one of these fields are set, then all of them should be set.
        // Here we only check if one of the fields is `Some`, which means all the other fields
        // _should_ also be `Some`, and let our database constraints catch any cases where they are not,
        // which will log an error that we can audit.
        if c.quantum_type_code.is_some()
            || s.living_weight.is_some()
            || s.species_fao_code.is_some()
            || s.species_fdir_code.is_some()
            || s.species_group_code.is_some()
            || s.species_main_group_code.is_some()
        {
            Some(Self {
                message_id: ers_tra.message_info.message_id as i64,
                ers_quantum_type_id: ers_tra.catch.quantum_type_code.as_deref(),
                living_weight: s.living_weight.map(|v| v as i32),
                species_fao_id: ers_tra.catch.species.species_fao_code.as_deref(),
                species_fiskeridir_id: ers_tra.catch.species.species_fdir_code.map(|v| v as i32),
                species_group_id: ers_tra
                    .catch
                    .species
                    .species_group_code
                    .unwrap_or(SpeciesGroup::Unknown) as i32,
                species_main_group_id: ers_tra
                    .catch
                    .species
                    .species_main_group_code
                    .unwrap_or(SpeciesMainGroup::Unknown)
                    as i32,
            })
        } else {
            None
        }
    }
}

impl TryFrom<Tra> for kyogre_core::Tra {
    type Error = Error;

    fn try_from(value: Tra) -> Result<Self, Self::Error> {
        let Tra {
            fiskeridir_vessel_id,
            latitude,
            longitude,
            reloading_timestamp,
            message_timestamp,
            catches,
            reload_to,
            reload_from,
            reload_to_call_sign,
            reload_from_call_sign,
        } = value;

        Ok(Self {
            latitude,
            longitude,
            reloading_timestamp,
            catches: serde_json::from_str::<Vec<TripTraCatch>>(&catches)?
                .into_iter()
                .map(kyogre_core::TraCatch::from)
                .collect(),
            reload_to_fiskeridir_vessel_id: reload_to,
            reload_from_fiskeridir_vessel_id: reload_from,
            fiskeridir_vessel_id,
            message_timestamp,
            reload_to_call_sign,
            reload_from_call_sign,
        })
    }
}

impl From<TripTra> for kyogre_core::Tra {
    fn from(value: TripTra) -> Self {
        let TripTra {
            fiskeridir_vessel_id,
            latitude,
            longitude,
            reloading_timestamp,
            message_timestamp,
            catches,
            reload_to,
            reload_from,
            reload_to_call_sign,
            reload_from_call_sign,
        } = value;

        Self {
            latitude,
            longitude,
            reloading_timestamp,
            catches: catches
                .into_iter()
                .map(kyogre_core::TraCatch::from)
                .collect(),
            reload_to_fiskeridir_vessel_id: reload_to,
            reload_from_fiskeridir_vessel_id: reload_from,
            fiskeridir_vessel_id,
            message_timestamp,
            reload_to_call_sign,
            reload_from_call_sign,
        }
    }
}

impl From<TripTraCatch> for kyogre_core::TraCatch {
    fn from(value: TripTraCatch) -> Self {
        let TripTraCatch {
            living_weight,
            species_group_id,
            catch_quantum,
        } = value;

        Self {
            living_weight,
            species_group_id,
            catch_quantum,
        }
    }
}
