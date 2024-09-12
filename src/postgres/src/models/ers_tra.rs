use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::{FiskdirVesselNationalityGroup, SpeciesGroup, SpeciesMainGroup};
use kyogre_core::FiskeridirVesselId;
use unnest_insert::UnnestInsert;

use crate::queries::{opt_type_to_i64, type_to_i32};

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "ers_tra",
    returning = "vessel_event_id, message_timestamp, reloading_timestamp, fiskeridir_vessel_id"
)]
pub struct NewErsTra {
    pub message_id: i64,
    pub message_number: i32,
    pub message_timestamp: DateTime<Utc>,
    pub ers_message_type_id: String,
    pub message_year: i32,
    pub relevant_year: i32,
    pub sequence_number: Option<i32>,
    pub reloading_timestamp: Option<DateTime<Utc>>,
    #[unnest_insert(sql_type = "BIGINT", type_conversion = "opt_type_to_i64")]
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
    pub vessel_building_year: Option<i32>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_engine_building_year: Option<i32>,
    pub vessel_engine_power: Option<i32>,
    pub vessel_gross_tonnage_1969: Option<i32>,
    pub vessel_gross_tonnage_other: Option<i32>,
    pub vessel_county: Option<String>,
    pub vessel_county_code: Option<i32>,
    pub vessel_greatest_length: Option<f64>,
    pub vessel_identification: String,
    pub vessel_length: f64,
    pub vessel_length_group: Option<String>,
    pub vessel_length_group_code: Option<i32>,
    pub vessel_material_code: Option<String>,
    pub vessel_municipality: Option<String>,
    pub vessel_municipality_code: Option<i32>,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub vessel_nationality_code: String,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub fiskeridir_vessel_nationality_group_id: FiskdirVesselNationalityGroup,
    pub vessel_rebuilding_year: Option<i32>,
    pub vessel_registration_id: Option<String>,
    pub vessel_registration_id_ers: Option<String>,
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
pub struct NewErsTraCatch {
    pub message_id: i64,
    pub ers_quantum_type_id: Option<String>,
    pub living_weight: Option<i32>,
    pub species_fao_id: Option<String>,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: i32,
    pub species_main_group_id: i32,
}

impl From<fiskeridir_rs::ErsTra> for NewErsTra {
    fn from(v: fiskeridir_rs::ErsTra) -> Self {
        Self {
            message_id: v.message_info.message_id as i64,
            message_number: v.message_info.message_number as i32,
            message_timestamp: v.message_info.message_timestamp(),
            reloading_timestamp: v.reloading_timestamp(),
            ers_message_type_id: v.message_info.message_type_code.into_inner(),
            message_year: v.message_info.message_year as i32,
            relevant_year: v.message_info.relevant_year as i32,
            sequence_number: v.message_info.sequence_number.map(|v| v as i32),
            fiskeridir_vessel_id: v.vessel_info.id,
            vessel_building_year: v.vessel_info.building_year.map(|v| v as i32),
            vessel_call_sign: v.vessel_info.call_sign.map(|v| v.into_inner()),
            vessel_call_sign_ers: v.vessel_info.call_sign_ers.into_inner(),
            vessel_engine_building_year: v.vessel_info.engine_building_year.map(|v| v as i32),
            vessel_engine_power: v.vessel_info.engine_power.map(|v| v as i32),
            vessel_gross_tonnage_1969: v.vessel_info.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: v.vessel_info.gross_tonnage_other.map(|v| v as i32),
            vessel_county: v.vessel_info.county.map(|v| v.into_inner()),
            vessel_county_code: v.vessel_info.county_code.map(|v| v as i32),
            vessel_greatest_length: v.vessel_info.greatest_length,
            vessel_identification: v.vessel_info.identification.into_inner(),
            vessel_length: v.vessel_info.length,
            vessel_length_group: v.vessel_info.length_group.map(|v| v.into_inner()),
            vessel_length_group_code: v.vessel_info.length_group_code.map(|v| v as i32),
            vessel_material_code: v.vessel_info.material_code.map(|v| v.into_inner()),
            vessel_municipality: v.vessel_info.municipality.map(|v| v.into_inner()),
            vessel_municipality_code: v.vessel_info.municipality_code.map(|v| v as i32),
            vessel_name: v.vessel_info.name.map(|v| v.into_inner()),
            vessel_name_ers: v.vessel_info.name_ers.map(|v| v.into_inner()),
            vessel_nationality_code: v.vessel_info.nationality_code.alpha3().to_string(),
            fiskeridir_vessel_nationality_group_id: v.vessel_info.nationality_group_code,
            vessel_rebuilding_year: v.vessel_info.rebuilding_year.map(|v| v as i32),
            vessel_registration_id: v.vessel_info.registration_id.map(|v| v.into_inner()),
            vessel_registration_id_ers: v.vessel_info.registration_id_ers.map(|v| v.into_inner()),
            vessel_valid_until: v.vessel_info.valid_until,
            vessel_valid_from: v.vessel_info.valid_from,
            vessel_width: v.vessel_info.width,
            vessel_event_id: None,
        }
    }
}

impl NewErsTraCatch {
    pub fn from_ers_tra(ers_tra: &fiskeridir_rs::ErsTra) -> Option<Self> {
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
                ers_quantum_type_id: ers_tra
                    .catch
                    .quantum_type_code
                    .clone()
                    .map(|v| v.into_inner()),
                living_weight: s.living_weight.map(|v| v as i32),
                species_fao_id: ers_tra
                    .catch
                    .species
                    .species_fao_code
                    .clone()
                    .map(|v| v.into_inner()),
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
