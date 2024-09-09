use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::{
    FiskdirVesselNationalityGroup, Gear, GearGroup, MainGearGroup, SpeciesGroup, SpeciesMainGroup,
    WhaleGender,
};
use kyogre_core::FiskeridirVesselId;
use unnest_insert::UnnestInsert;

use crate::{
    error::Error,
    queries::{
        opt_timestamp_from_date_and_time, opt_type_to_i32, opt_type_to_i64,
        timestamp_from_date_and_time, type_to_i32,
    },
};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ers_dca", returning = "message_id,vessel_event_id")]
pub struct NewErsDca {
    pub message_id: i64,
    pub message_number: i32,
    pub message_timestamp: DateTime<Utc>,
    pub ers_message_type_id: String,
    pub message_version: i32,
    pub message_year: i32,
    pub relevant_year: i32,
    pub sequence_number: Option<i32>,
    pub ers_activity_id: String,
    pub quota_type_id: i32,
    pub port_id: Option<String>,
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
    pub vessel_valid_from: Option<NaiveDate>,
    pub vessel_valid_until: Option<NaiveDate>,
    pub vessel_width: Option<f64>,
    pub vessel_event_id: Option<i64>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ers_dca_bodies")]
pub struct NewErsDcaBody {
    pub message_id: i64,
    pub start_latitude: Option<f64>,
    pub start_longitude: Option<f64>,
    pub start_timestamp: Option<DateTime<Utc>>,
    pub stop_latitude: Option<f64>,
    pub stop_longitude: Option<f64>,
    pub stop_timestamp: Option<DateTime<Utc>>,
    pub ocean_depth_end: Option<i32>,
    pub ocean_depth_start: Option<i32>,
    pub location_end_code: Option<i32>,
    pub location_start_code: Option<i32>,
    pub area_grouping_end_id: Option<String>,
    pub area_grouping_start_id: Option<String>,
    pub main_area_end_id: Option<i32>,
    pub main_area_start_id: Option<i32>,
    pub duration: Option<i32>,
    pub haul_distance: Option<i32>,
    pub call_sign_of_loading_vessel: Option<String>,
    pub catch_year: Option<i32>,
    pub economic_zone_id: Option<String>,
    pub gear_amount: Option<i32>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub gear_id: Gear,
    pub gear_fao_id: Option<String>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub gear_group_id: GearGroup,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub gear_main_group_id: MainGearGroup,
    pub gear_mesh_width: Option<i32>,
    pub gear_problem_id: Option<i32>,
    pub gear_specification_id: Option<i32>,
    pub herring_population_id: Option<String>,
    pub herring_population_fiskeridir_id: Option<i32>,
    pub majority_species_fao_id: Option<String>,
    pub majority_species_fiskeridir_id: Option<i32>,
    pub living_weight: Option<i32>,
    pub species_fao_id: Option<String>,
    pub species_fiskeridir_id: Option<i32>,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub species_group_id: SpeciesGroup,
    #[unnest_insert(sql_type = "INT", type_conversion = "type_to_i32")]
    pub species_main_group_id: SpeciesMainGroup,
    pub whale_grenade_number: Option<String>,
    pub whale_blubber_measure_a: Option<i32>,
    pub whale_blubber_measure_b: Option<i32>,
    pub whale_blubber_measure_c: Option<i32>,
    pub whale_circumference: Option<i32>,
    pub whale_fetus_length: Option<i32>,
    #[unnest_insert(sql_type = "INT", type_conversion = "opt_type_to_i32")]
    pub whale_gender_id: Option<WhaleGender>,
    pub whale_individual_number: Option<i32>,
    pub whale_length: Option<i32>,
}

impl TryFrom<fiskeridir_rs::ErsDca> for NewErsDca {
    type Error = Error;

    fn try_from(v: fiskeridir_rs::ErsDca) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: v.message_info.message_id as i64,
            message_number: v.message_info.message_number as i32,
            message_timestamp: timestamp_from_date_and_time(
                v.message_info.message_date,
                v.message_info.message_time,
            ),
            ers_message_type_id: v.message_info.message_type_code.into_inner(),
            message_version: v.message_version as i32,
            message_year: v.message_info.message_year as i32,
            relevant_year: v.message_info.relevant_year as i32,
            sequence_number: v.message_info.sequence_number.map(|v| v as i32),
            ers_activity_id: v.activity_code.into_inner(),
            quota_type_id: v.quota_type_code as i32,
            port_id: v.port.code,
            fiskeridir_vessel_id: v.vessel_info.vessel_id,
            vessel_building_year: v.vessel_info.building_year.map(|v| v as i32),
            vessel_call_sign: v.vessel_info.call_sign,
            vessel_call_sign_ers: v.vessel_info.call_sign_ers.into_inner(),
            vessel_engine_building_year: v.vessel_info.engine_building_year.map(|v| v as i32),
            vessel_engine_power: v.vessel_info.engine_power.map(|v| v as i32),
            vessel_gross_tonnage_1969: v.vessel_info.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: v.vessel_info.gross_tonnage_other.map(|v| v as i32),
            vessel_county: v.vessel_info.vessel_county,
            vessel_county_code: v.vessel_info.vessel_county_code.map(|v| v as i32),
            vessel_greatest_length: v.vessel_info.vessel_greatest_length,
            vessel_identification: v.vessel_info.vessel_identification.into_inner(),
            vessel_length: v.vessel_info.vessel_length,
            vessel_length_group: v.vessel_info.vessel_length_group,
            vessel_length_group_code: v.vessel_info.vessel_length_group_code.map(|v| v as i32),
            vessel_material_code: v.vessel_info.vessel_material_code,
            vessel_municipality: v.vessel_info.vessel_municipality,
            vessel_municipality_code: v.vessel_info.vessel_municipality_code.map(|v| v as i32),
            vessel_name: v.vessel_info.vessel_name,
            vessel_name_ers: v.vessel_info.vessel_name_ers,
            vessel_nationality_code: v.vessel_info.vessel_nationality_code.into_inner(),
            fiskeridir_vessel_nationality_group_id: v.vessel_info.vessel_nationality_group_code,
            vessel_rebuilding_year: v.vessel_info.vessel_rebuilding_year.map(|v| v as i32),
            vessel_registration_id: v.vessel_info.vessel_registration_id,
            vessel_registration_id_ers: v.vessel_info.vessel_registration_id_ers,
            vessel_valid_from: v.vessel_info.vessel_valid_from,
            vessel_valid_until: v.vessel_info.vessel_valid_until,
            vessel_width: v.vessel_info.vessel_width,
            vessel_event_id: None,
        })
    }
}

impl TryFrom<&fiskeridir_rs::ErsDca> for NewErsDcaBody {
    type Error = Error;

    fn try_from(v: &fiskeridir_rs::ErsDca) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: v.message_info.message_id as i64,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: opt_timestamp_from_date_and_time(v.start_date, v.start_time),
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: opt_timestamp_from_date_and_time(v.stop_date, v.stop_time),
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            location_end_code: v.location_end_code.map(|v| v as i32),
            location_start_code: v.location_start_code.map(|v| v as i32),
            area_grouping_end_id: v.area_grouping_end_code.clone(),
            area_grouping_start_id: v.area_grouping_start_code.clone(),
            main_area_end_id: v.main_area_end_code.map(|v| v as i32),
            main_area_start_id: v.main_area_start_code.map(|v| v as i32),
            duration: v.duration.map(|v| v as i32),
            haul_distance: v.haul_distance.map(|v| v as i32),
            call_sign_of_loading_vessel: v.call_sign_of_loading_vessel.clone(),
            catch_year: v.catch_year.map(|v| v as i32),
            economic_zone_id: v.economic_zone_code.clone(),
            gear_amount: v.gear.gear_amount.map(|v| v as i32),
            gear_id: v.gear.gear_fdir_code.unwrap_or(Gear::Unknown),
            gear_fao_id: v.gear.gear_fao_code.clone(),
            gear_group_id: v.gear.gear_group_code.unwrap_or(GearGroup::Unknown),
            gear_main_group_id: v
                .gear
                .gear_main_group_code
                .unwrap_or(MainGearGroup::Unknown),
            gear_mesh_width: v.gear.gear_mesh_width.map(|v| v as i32),
            gear_problem_id: v.gear.gear_problem_code.map(|v| v as i32),
            gear_specification_id: v.gear.gear_specification_code.map(|v| v as i32),
            herring_population_id: v.herring_population_code.clone(),
            herring_population_fiskeridir_id: v.herring_population_fdir_code.map(|v| v as i32),
            majority_species_fao_id: v.catch.majority_species_fao_code.clone(),
            majority_species_fiskeridir_id: v.catch.majority_species_fdir_code.map(|v| v as i32),
            living_weight: v.catch.species.living_weight.map(|v| v as i32),
            species_fao_id: v.catch.species.species_fao_code.clone(),
            species_fiskeridir_id: v.catch.species.species_fdir_code.map(|v| v as i32),
            species_group_id: v
                .catch
                .species
                .species_group_code
                .unwrap_or(SpeciesGroup::Unknown),
            species_main_group_id: v
                .catch
                .species
                .species_main_group_code
                .unwrap_or(SpeciesMainGroup::Unknown),
            whale_grenade_number: v.whale_catch_info.grenade_number.clone(),
            whale_blubber_measure_a: v.whale_catch_info.blubber_measure_a.map(|v| v as i32),
            whale_blubber_measure_b: v.whale_catch_info.blubber_measure_b.map(|v| v as i32),
            whale_blubber_measure_c: v.whale_catch_info.blubber_measure_c.map(|v| v as i32),
            whale_circumference: v.whale_catch_info.circumference.map(|v| v as i32),
            whale_fetus_length: v.whale_catch_info.fetus_length.map(|v| v as i32),
            whale_gender_id: v.whale_catch_info.gender_code,
            whale_individual_number: v.whale_catch_info.individual_number.map(|v| v as i32),
            whale_length: v.whale_catch_info.length.map(|v| v as i32),
        })
    }
}
