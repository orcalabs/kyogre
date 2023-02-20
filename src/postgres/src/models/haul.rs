use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use error_stack::{report, IntoReport, Report, ResultExt};
use kyogre_core::{FiskdirVesselNationalityGroup, WhaleGender};
use serde::Deserialize;

use crate::{
    error::{PostgresError, WhaleGenderError},
    queries::{decimal_to_float, opt_decimal_to_float},
};

pub struct Haul {
    pub message_id: i64,
    pub message_date: NaiveDate,
    pub message_number: i32,
    pub message_time: NaiveTime,
    pub message_timestamp: DateTime<Utc>,
    pub ers_message_type_id: String,
    pub message_year: i32,
    pub relevant_year: i32,
    pub sequence_number: Option<i32>,
    pub message_version: i32,
    pub ers_activity_id: String,
    pub area_grouping_end_id: Option<String>,
    pub area_grouping_start_id: Option<String>,
    pub call_sign_of_loading_vessel: Option<String>,
    pub catch_year: Option<i32>,
    pub duration: Option<i32>,
    pub economic_zone_id: Option<String>,
    pub haul_distance: Option<i32>,
    pub herring_population_id: Option<String>,
    pub herring_population_fiskeridir_id: Option<i32>,
    pub location_end_code: Option<i32>,
    pub location_start_code: Option<i32>,
    pub main_area_end_id: Option<i32>,
    pub main_area_start_id: Option<i32>,
    pub ocean_depth_end: Option<i32>,
    pub ocean_depth_start: Option<i32>,
    pub quota_type_id: i32,
    pub start_date: Option<NaiveDate>,
    pub start_latitude: Option<BigDecimal>,
    pub start_longitude: Option<BigDecimal>,
    pub start_time: Option<NaiveTime>,
    pub start_timestamp: Option<DateTime<Utc>>,
    pub stop_date: Option<NaiveDate>,
    pub stop_latitude: Option<BigDecimal>,
    pub stop_longitude: Option<BigDecimal>,
    pub stop_time: Option<NaiveTime>,
    pub stop_timestamp: Option<DateTime<Utc>>,
    pub gear_amount: Option<i32>,
    pub gear_fao_id: Option<String>,
    pub gear_fiskeridir_id: Option<i32>,
    pub gear_group_id: Option<i32>,
    pub gear_main_group_id: Option<i32>,
    pub gear_mesh_width: Option<i32>,
    pub gear_problem_id: Option<i32>,
    pub gear_specification_id: Option<i32>,
    pub port_id: Option<String>,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_building_year: Option<i32>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_engine_building_year: Option<i32>,
    pub vessel_engine_power: Option<i32>,
    pub vessel_gross_tonnage_1969: Option<i32>,
    pub vessel_gross_tonnage_other: Option<i32>,
    pub vessel_county: Option<String>,
    pub vessel_county_code: Option<i32>,
    pub vessel_greatest_length: Option<BigDecimal>,
    pub vessel_identification: String,
    pub vessel_length: BigDecimal,
    pub vessel_length_group: Option<String>,
    pub vessel_length_group_code: Option<i32>,
    pub vessel_material_code: Option<String>,
    pub vessel_municipality: Option<String>,
    pub vessel_municipality_code: Option<i32>,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub vessel_nationality_code: String,
    pub vessel_nationality_group_id: i32,
    pub vessel_rebuilding_year: Option<i32>,
    pub vessel_registration_id: Option<String>,
    pub vessel_registration_id_ers: Option<String>,
    pub vessel_valid_until: Option<NaiveDate>,
    pub vessel_width: Option<BigDecimal>,
    pub catches: serde_json::Value,
    pub whale_catches: serde_json::Value,
}

#[derive(Deserialize)]
pub struct HaulCatch {
    pub main_species_fao_id: Option<String>,
    pub main_species_fiskeridir_id: Option<i32>,
    pub living_weight: Option<i32>,
    pub species_fao_id: Option<String>,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: Option<i32>,
    pub species_main_group_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    pub gender_id: Option<i32>,
    pub grenade_number: Option<String>,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}

impl TryFrom<Haul> for kyogre_core::Haul {
    type Error = Report<PostgresError>;

    fn try_from(v: Haul) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: v.message_id,
            message_date: v.message_date,
            message_number: v.message_number,
            message_time: v.message_time,
            message_timestamp: v.message_timestamp,
            ers_message_type_id: v.ers_message_type_id,
            message_year: v.message_year,
            relevant_year: v.relevant_year,
            sequence_number: v.sequence_number,
            message_version: v.message_version,
            ers_activity_id: v.ers_activity_id,
            area_grouping_end_id: v.area_grouping_end_id,
            area_grouping_start_id: v.area_grouping_start_id,
            call_sign_of_loading_vessel: v.call_sign_of_loading_vessel,
            catch_year: v.catch_year,
            duration: v.duration,
            economic_zone_id: v.economic_zone_id,
            haul_distance: v.haul_distance,
            herring_population_id: v.herring_population_id,
            herring_population_fiskeridir_id: v.herring_population_fiskeridir_id,
            location_end_code: v.location_end_code,
            location_start_code: v.location_start_code,
            main_area_end_id: v.main_area_end_id,
            main_area_start_id: v.main_area_start_id,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_date: v.start_date,
            start_latitude: opt_decimal_to_float(v.start_latitude)
                .change_context(PostgresError::DataConversion)?,
            start_longitude: opt_decimal_to_float(v.start_longitude)
                .change_context(PostgresError::DataConversion)?,
            start_time: v.start_time,
            start_timestamp: v.start_timestamp,
            stop_date: v.stop_date,
            stop_latitude: opt_decimal_to_float(v.stop_latitude)
                .change_context(PostgresError::DataConversion)?,
            stop_longitude: opt_decimal_to_float(v.stop_longitude)
                .change_context(PostgresError::DataConversion)?,
            stop_time: v.stop_time,
            stop_timestamp: v.stop_timestamp,
            gear_amount: v.gear_amount,
            gear_fao_id: v.gear_fao_id,
            gear_fiskeridir_id: v.gear_fiskeridir_id,
            gear_group_id: v.gear_group_id,
            gear_main_group_id: v.gear_main_group_id,
            gear_mesh_width: v.gear_mesh_width,
            gear_problem_id: v.gear_problem_id,
            gear_specification_id: v.gear_specification_id,
            port_id: v.port_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_building_year: v.vessel_building_year,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_engine_building_year: v.vessel_engine_building_year,
            vessel_engine_power: v.vessel_engine_power,
            vessel_gross_tonnage_1969: v.vessel_gross_tonnage_1969,
            vessel_gross_tonnage_other: v.vessel_gross_tonnage_other,
            vessel_county: v.vessel_county,
            vessel_county_code: v.vessel_county_code,
            vessel_greatest_length: opt_decimal_to_float(v.vessel_greatest_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_identification: v.vessel_identification,
            vessel_length: decimal_to_float(v.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_length_group: v.vessel_length_group,
            vessel_length_group_code: v.vessel_length_group_code,
            vessel_material_code: v.vessel_material_code,
            vessel_municipality: v.vessel_municipality,
            vessel_municipality_code: v.vessel_municipality_code,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            vessel_nationality_code: v.vessel_nationality_code,
            vessel_nationality_group_id: FiskdirVesselNationalityGroup::from_i32(
                v.vessel_nationality_group_id,
            )
            .ok_or_else(|| report!(PostgresError::DataConversion))?,
            vessel_rebuilding_year: v.vessel_rebuilding_year,
            vessel_registration_id: v.vessel_registration_id,
            vessel_registration_id_ers: v.vessel_registration_id_ers,
            vessel_valid_until: v.vessel_valid_until,
            vessel_width: opt_decimal_to_float(v.vessel_width)
                .change_context(PostgresError::DataConversion)?,
            catches: serde_json::from_value::<Vec<HaulCatch>>(v.catches)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::HaulCatch::try_from)
                .collect::<Result<Vec<_>, Report<PostgresError>>>()?,
            whale_catches: serde_json::from_value::<Vec<WhaleCatch>>(v.whale_catches)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::WhaleCatch::try_from)
                .collect::<Result<Vec<_>, Report<PostgresError>>>()?,
        })
    }
}

impl TryFrom<HaulCatch> for kyogre_core::HaulCatch {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulCatch) -> Result<Self, Self::Error> {
        Ok(Self {
            main_species_fao_id: v.main_species_fao_id,
            main_species_fiskeridir_id: v.main_species_fiskeridir_id,
            living_weight: v.living_weight,
            species_fao_id: v.species_fao_id,
            species_fiskeridir_id: v.species_fiskeridir_id,
            species_group_id: v.species_group_id,
            species_main_group_id: v.species_main_group_id,
        })
    }
}

impl TryFrom<WhaleCatch> for kyogre_core::WhaleCatch {
    type Error = Report<PostgresError>;

    fn try_from(v: WhaleCatch) -> Result<Self, Self::Error> {
        Ok(Self {
            blubber_measure_a: v.blubber_measure_a,
            blubber_measure_b: v.blubber_measure_b,
            blubber_measure_c: v.blubber_measure_c,
            circumference: v.circumference,
            fetus_length: v.fetus_length,
            gender_id: v
                .gender_id
                .map(|g| {
                    WhaleGender::from_i32(g)
                        .ok_or(WhaleGenderError(g))
                        .into_report()
                        .change_context(PostgresError::DataConversion)
                })
                .transpose()?,
            grenade_number: v.grenade_number,
            individual_number: v.individual_number,
            length: v.length,
        })
    }
}
