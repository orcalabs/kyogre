use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use error_stack::{report, Report, ResultExt};
use kyogre_core::FiskdirVesselNationalityGroup;
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    queries::{
        enum_to_i32, float_to_decimal, opt_float_to_decimal, opt_timestamp_from_date_and_time,
    },
};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "ers_dca",
    conflict = "message_id,start_timestamp,stop_timestamp"
)]
pub struct NewErsDca {
    pub message_id: i64,
    pub message_number: i32,
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
    pub start_latitude: Option<BigDecimal>,
    pub start_longitude: Option<BigDecimal>,
    pub start_timestamp: DateTime<Utc>,
    pub stop_latitude: Option<BigDecimal>,
    pub stop_longitude: Option<BigDecimal>,
    pub stop_timestamp: DateTime<Utc>,
    pub gear_amount: Option<i32>,
    pub gear_fao_id: Option<String>,
    pub gear_id: i32,
    pub gear_group_id: i32,
    pub gear_main_group_id: i32,
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
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub fiskeridir_vessel_nationality_group_id: FiskdirVesselNationalityGroup,
    pub vessel_rebuilding_year: Option<i32>,
    pub vessel_registration_id: Option<String>,
    pub vessel_registration_id_ers: Option<String>,
    pub vessel_valid_until: Option<NaiveDate>,
    pub vessel_width: Option<BigDecimal>,
    pub majority_species_fao_id: Option<String>,
    pub majority_species_fiskeridir_id: Option<i32>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "ers_dca_other", conflict = "message_id")]
pub struct NewErsDcaOther {
    pub message_id: i64,
    pub message_number: i32,
    pub message_timestamp: DateTime<Utc>,
    pub ers_message_type_id: String,
    pub message_year: i32,
    pub relevant_year: i32,
    pub sequence_number: Option<i32>,
    pub message_version: i32,
    pub ers_activity_id: String,
    pub quota_type_id: i32,
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
    #[unnest_insert(sql_type = "INT", type_conversion = "enum_to_i32")]
    pub fiskeridir_vessel_nationality_group_id: FiskdirVesselNationalityGroup,
    pub vessel_rebuilding_year: Option<i32>,
    pub vessel_registration_id: Option<String>,
    pub vessel_registration_id_ers: Option<String>,
    pub vessel_valid_until: Option<NaiveDate>,
    pub vessel_width: Option<BigDecimal>,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "ers_dca_catches",
    conflict = "message_id,start_timestamp,stop_timestamp,species_fao_id"
)]
pub struct NewErsDcaCatch {
    pub message_id: i64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    pub message_version: i32,
    pub living_weight: Option<i32>,
    pub species_fao_id: String,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: i32,
    pub species_main_group_id: i32,
}

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(
    table_name = "ers_dca_whale_catches",
    conflict = "message_id,start_timestamp,stop_timestamp,whale_grenade_number"
)]
pub struct NewErsDcaWhaleCatch {
    pub message_id: i64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    pub message_version: i32,
    pub whale_grenade_number: String,
    pub whale_blubber_measure_a: Option<i32>,
    pub whale_blubber_measure_b: Option<i32>,
    pub whale_blubber_measure_c: Option<i32>,
    pub whale_circumference: Option<i32>,
    pub whale_fetus_length: Option<i32>,
    pub whale_gender_id: Option<i32>,
    pub whale_individual_number: Option<i32>,
    pub whale_length: Option<i32>,
}

impl TryFrom<fiskeridir_rs::ErsDca> for NewErsDca {
    type Error = Report<PostgresError>;

    fn try_from(v: fiskeridir_rs::ErsDca) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: v.message_info.message_id as i64,
            message_number: v.message_info.message_number as i32,
            message_timestamp: DateTime::<Utc>::from_utc(
                v.message_info
                    .message_date
                    .and_time(v.message_info.message_time),
                Utc,
            ),
            ers_message_type_id: v.message_info.message_type_code.into_inner(),
            message_year: v.message_info.message_year as i32,
            relevant_year: v.message_info.relevant_year as i32,
            sequence_number: v.message_info.sequence_number.map(|v| v as i32),
            message_version: v.message_version as i32,
            ers_activity_id: v.activity_code.into_inner(),
            area_grouping_end_id: v.area_grouping_end_code,
            area_grouping_start_id: v.area_grouping_start_code,
            call_sign_of_loading_vessel: v.call_sign_of_loading_vessel,
            catch_year: v.catch_year.map(|v| v as i32),
            duration: v.duration.map(|v| v as i32),
            economic_zone_id: v.economic_zone_code,
            haul_distance: v.haul_distance.map(|v| v as i32),
            herring_population_id: v.herring_population_code,
            herring_population_fiskeridir_id: v.herring_population_fdir_code.map(|v| v as i32),
            location_end_code: v.location_end_code.map(|v| v as i32),
            location_start_code: v.location_start_code.map(|v| v as i32),
            main_area_end_id: v.main_area_end_code.map(|v| v as i32),
            main_area_start_id: v.main_area_start_code.map(|v| v as i32),
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_code as i32,
            start_latitude: opt_float_to_decimal(v.start_latitude)
                .change_context(PostgresError::DataConversion)?,
            start_longitude: opt_float_to_decimal(v.start_longitude)
                .change_context(PostgresError::DataConversion)?,
            start_timestamp: opt_timestamp_from_date_and_time(v.start_date, v.start_time)
                .ok_or_else(|| {
                    report!(PostgresError::DataConversion)
                        .attach_printable("expected start_timestamp to be `Some` for ers_dca")
                })?,
            stop_latitude: opt_float_to_decimal(v.stop_latitude)
                .change_context(PostgresError::DataConversion)?,
            stop_longitude: opt_float_to_decimal(v.stop_longitude)
                .change_context(PostgresError::DataConversion)?,
            stop_timestamp: opt_timestamp_from_date_and_time(v.stop_date, v.stop_time).ok_or_else(
                || {
                    report!(PostgresError::DataConversion)
                        .attach_printable("expected stop_timestamp to be `Some` for ers_dca")
                },
            )?,
            gear_amount: v.gear.gear_amount.map(|v| v as i32),
            gear_fao_id: v.gear.gear_fao_code,
            gear_id: v.gear.gear_fdir_code as i32,
            gear_group_id: v.gear.gear_group_code as i32,
            gear_main_group_id: v.gear.gear_main_group_code as i32,
            gear_mesh_width: v.gear.gear_mesh_width.map(|v| v as i32),
            gear_problem_id: v.gear.gear_problem_code.map(|v| v as i32),
            gear_specification_id: v.gear.gear_specification_code.map(|v| v as i32),
            port_id: v.port.code,
            fiskeridir_vessel_id: v.vessel_info.vessel_id.map(|v| v as i64),
            vessel_building_year: v.vessel_info.building_year.map(|v| v as i32),
            vessel_call_sign: v.vessel_info.call_sign,
            vessel_call_sign_ers: v.vessel_info.call_sign_ers.into_inner(),
            vessel_engine_building_year: v.vessel_info.engine_building_year.map(|v| v as i32),
            vessel_engine_power: v.vessel_info.engine_power.map(|v| v as i32),
            vessel_gross_tonnage_1969: v.vessel_info.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: v.vessel_info.gross_tonnage_other.map(|v| v as i32),
            vessel_county: v.vessel_info.vessel_county,
            vessel_county_code: v.vessel_info.vessel_county_code.map(|v| v as i32),
            vessel_greatest_length: opt_float_to_decimal(v.vessel_info.vessel_greatest_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_identification: v.vessel_info.vessel_identification.into_inner(),
            vessel_length: float_to_decimal(v.vessel_info.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_length_group: v.vessel_info.vessel_length_group,
            vessel_length_group_code: v.vessel_info.vessel_length_group_code.map(|v| v as i32),
            vessel_material_code: v.vessel_info.vessel_material_code,
            vessel_municipality: v.vessel_info.vessel_municipality,
            vessel_municipality_code: v.vessel_info.vessel_municipality_code.map(|v| v as i32),
            vessel_name: v.vessel_info.vessel_name,
            vessel_name_ers: v.vessel_info.vessel_name_ers,
            vessel_nationality_code: v.vessel_info.vessel_nationality_code.into_inner(),
            fiskeridir_vessel_nationality_group_id: v
                .vessel_info
                .vessel_nationality_group_code
                .into(),
            vessel_rebuilding_year: v.vessel_info.vessel_rebuilding_year.map(|v| v as i32),
            vessel_registration_id: v.vessel_info.vessel_registration_id,
            vessel_registration_id_ers: v.vessel_info.vessel_registration_id_ers,
            vessel_valid_until: v.vessel_info.vessel_valid_until,
            vessel_width: opt_float_to_decimal(v.vessel_info.vessel_width)
                .change_context(PostgresError::DataConversion)?,
            majority_species_fao_id: v.catch.majority_species_fao_code.clone(),
            majority_species_fiskeridir_id: v.catch.majority_species_fdir_code.map(|v| v as i32),
        })
    }
}

impl TryFrom<fiskeridir_rs::ErsDca> for NewErsDcaOther {
    type Error = Report<PostgresError>;

    fn try_from(v: fiskeridir_rs::ErsDca) -> Result<Self, Self::Error> {
        Ok(Self {
            message_id: v.message_info.message_id as i64,
            message_number: v.message_info.message_number as i32,
            message_timestamp: DateTime::<Utc>::from_utc(
                v.message_info
                    .message_date
                    .and_time(v.message_info.message_time),
                Utc,
            ),
            ers_message_type_id: v.message_info.message_type_code.into_inner(),
            message_year: v.message_info.message_year as i32,
            relevant_year: v.message_info.relevant_year as i32,
            sequence_number: v.message_info.sequence_number.map(|v| v as i32),
            message_version: v.message_version as i32,
            ers_activity_id: v.activity_code.into_inner(),
            quota_type_id: v.quota_type_code as i32,
            port_id: v.port.code,
            fiskeridir_vessel_id: v.vessel_info.vessel_id.map(|v| v as i64),
            vessel_building_year: v.vessel_info.building_year.map(|v| v as i32),
            vessel_call_sign: v.vessel_info.call_sign,
            vessel_call_sign_ers: v.vessel_info.call_sign_ers.into_inner(),
            vessel_engine_building_year: v.vessel_info.engine_building_year.map(|v| v as i32),
            vessel_engine_power: v.vessel_info.engine_power.map(|v| v as i32),
            vessel_gross_tonnage_1969: v.vessel_info.gross_tonnage_1969.map(|v| v as i32),
            vessel_gross_tonnage_other: v.vessel_info.gross_tonnage_other.map(|v| v as i32),
            vessel_county: v.vessel_info.vessel_county,
            vessel_county_code: v.vessel_info.vessel_county_code.map(|v| v as i32),
            vessel_greatest_length: opt_float_to_decimal(v.vessel_info.vessel_greatest_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_identification: v.vessel_info.vessel_identification.into_inner(),
            vessel_length: float_to_decimal(v.vessel_info.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_length_group: v.vessel_info.vessel_length_group,
            vessel_length_group_code: v.vessel_info.vessel_length_group_code.map(|v| v as i32),
            vessel_material_code: v.vessel_info.vessel_material_code,
            vessel_municipality: v.vessel_info.vessel_municipality,
            vessel_municipality_code: v.vessel_info.vessel_municipality_code.map(|v| v as i32),
            vessel_name: v.vessel_info.vessel_name,
            vessel_name_ers: v.vessel_info.vessel_name_ers,
            vessel_nationality_code: v.vessel_info.vessel_nationality_code.into_inner(),
            fiskeridir_vessel_nationality_group_id: v
                .vessel_info
                .vessel_nationality_group_code
                .into(),
            vessel_rebuilding_year: v.vessel_info.vessel_rebuilding_year.map(|v| v as i32),
            vessel_registration_id: v.vessel_info.vessel_registration_id,
            vessel_registration_id_ers: v.vessel_info.vessel_registration_id_ers,
            vessel_valid_until: v.vessel_info.vessel_valid_until,
            vessel_width: opt_float_to_decimal(v.vessel_info.vessel_width)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}

impl NewErsDcaCatch {
    pub fn from_ers_dca(
        ers_dca: &fiskeridir_rs::ErsDca,
    ) -> Result<Option<Self>, Report<PostgresError>> {
        let s = &ers_dca.catch.species;

        s.species_fao_code
            .as_ref()
            .map(|species_fao_id| {
                Ok(Self {
                    message_id: ers_dca.message_info.message_id as i64,
                    start_timestamp: opt_timestamp_from_date_and_time(
                        ers_dca.start_date,
                        ers_dca.start_time,
                    )
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion)
                            .attach_printable("expected start_timestamp to be `Some` for catch")
                    })?,
                    stop_timestamp: opt_timestamp_from_date_and_time(
                        ers_dca.stop_date,
                        ers_dca.stop_time,
                    )
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion)
                            .attach_printable("expected stop_timestamp to be `Some` for catch")
                    })?,
                    message_version: ers_dca.message_version as i32,
                    living_weight: s.living_weight.map(|ers_dca| ers_dca as i32),
                    species_fao_id: species_fao_id.clone(),
                    species_fiskeridir_id: s.species_fdir_code.map(|v| v as i32),
                    species_group_id: s.species_group_code as i32,
                    species_main_group_id: s.species_main_group_code as i32,
                })
            })
            .transpose()
    }
}

impl NewErsDcaWhaleCatch {
    pub fn from_ers_dca(
        ers_dca: &fiskeridir_rs::ErsDca,
    ) -> Result<Option<Self>, Report<PostgresError>> {
        let w = &ers_dca.whale_catch_info;

        w.grenade_number
            .as_ref()
            .map(|whale_grenade_number| {
                Ok(Self {
                    message_id: ers_dca.message_info.message_id as i64,
                    start_timestamp: opt_timestamp_from_date_and_time(
                        ers_dca.start_date,
                        ers_dca.start_time,
                    )
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion).attach_printable(
                            "expected start_timestamp to be `Some` for whale_catch",
                        )
                    })?,
                    stop_timestamp: opt_timestamp_from_date_and_time(
                        ers_dca.stop_date,
                        ers_dca.stop_time,
                    )
                    .ok_or_else(|| {
                        report!(PostgresError::DataConversion).attach_printable(
                            "expected stop_timestamp to be `Some` for whale_catch",
                        )
                    })?,
                    message_version: ers_dca.message_version as i32,
                    whale_blubber_measure_a: w.blubber_measure_a.map(|v| v as i32),
                    whale_blubber_measure_b: w.blubber_measure_b.map(|v| v as i32),
                    whale_blubber_measure_c: w.blubber_measure_c.map(|v| v as i32),
                    whale_circumference: w.circumference.map(|v| v as i32),
                    whale_fetus_length: w.fetus_length.map(|v| v as i32),
                    whale_gender_id: w.gender_code.map(|v| v as i32),
                    whale_grenade_number: whale_grenade_number.clone(),
                    whale_individual_number: w.individual_number.map(|v| v as i32),
                    whale_length: w.length.map(|v| v as i32),
                })
            })
            .transpose()
    }
}
