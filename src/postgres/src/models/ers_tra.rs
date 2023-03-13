use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use error_stack::{report, Report, ResultExt};
use kyogre_core::FiskdirVesselNationalityGroup;

use crate::{
    error::PostgresError,
    queries::{float_to_decimal, opt_float_to_decimal},
};

pub struct NewErsTra {
    pub message_id: i64,
    pub message_number: i32,
    pub message_timestamp: DateTime<Utc>,
    pub ers_message_type_id: String,
    pub message_year: i32,
    pub relevant_year: i32,
    pub sequence_number: Option<i32>,
    pub reloading_timestamp: Option<DateTime<Utc>>,
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
    pub vessel_nationality_group_id: FiskdirVesselNationalityGroup,
    pub vessel_rebuilding_year: Option<i32>,
    pub vessel_registration_id: Option<String>,
    pub vessel_registration_id_ers: Option<String>,
    pub vessel_valid_until: Option<NaiveDate>,
    pub vessel_valid_from: Option<NaiveDate>,
    pub vessel_width: Option<BigDecimal>,
}

pub struct NewErsTraCatch {
    pub message_id: i64,
    pub ers_quantum_type_id: Option<String>,
    pub living_weight: Option<i32>,
    pub species_fao_id: Option<String>,
    pub species_fiskeridir_id: Option<i32>,
    pub species_group_id: Option<i32>,
    pub species_main_group_id: Option<i32>,
}

impl TryFrom<fiskeridir_rs::ErsTra> for NewErsTra {
    type Error = Report<PostgresError>;

    fn try_from(v: fiskeridir_rs::ErsTra) -> Result<Self, Self::Error> {
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
            reloading_timestamp: v
                .reloading_date
                .map::<Result<_, Report<PostgresError>>, _>(|reloading_date| {
                    Ok(DateTime::<Utc>::from_utc(
                        reloading_date.and_time(v.reloading_time.ok_or_else(|| {
                            report!(PostgresError::DataConversion).attach_printable(
                                "expected reloading_time to be `Some` due to reloading_date",
                            )
                        })?),
                        Utc,
                    ))
                })
                .transpose()?,
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
            vessel_nationality_group_id: v.vessel_info.vessel_nationality_group_code.into(),
            vessel_rebuilding_year: v.vessel_info.vessel_rebuilding_year.map(|v| v as i32),
            vessel_registration_id: v.vessel_info.vessel_registration_id,
            vessel_registration_id_ers: v.vessel_info.vessel_registration_id_ers,
            vessel_valid_until: v.vessel_info.vessel_valid_until,
            vessel_valid_from: v.vessel_info.vessel_valid_from,
            vessel_width: opt_float_to_decimal(v.vessel_info.vessel_width)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}

impl NewErsTraCatch {
    pub fn from_ers_tra(ers_tra: &fiskeridir_rs::ErsTra) -> Option<Self> {
        let c = &ers_tra.catch;
        let s = &c.species;

        if c.quantum_type_code.is_some()
            || s.living_weight.is_some()
            || s.species_fao_code.is_some()
            || s.species_fdir_code.is_some()
            || s.species_group_code.is_some()
            || s.species_main_group_code.is_some()
        {
            Some(Self {
                message_id: ers_tra.message_info.message_id as i64,
                ers_quantum_type_id: ers_tra.catch.quantum_type_code.clone(),
                living_weight: s.living_weight.map(|v| v as i32),
                species_fao_id: ers_tra.catch.species.species_fao_code.clone(),
                species_fiskeridir_id: ers_tra.catch.species.species_fdir_code.map(|v| v as i32),
                species_group_id: ers_tra.catch.species.species_group_code.map(|v| v as i32),
                species_main_group_id: ers_tra
                    .catch
                    .species
                    .species_main_group_code
                    .map(|v| v as i32),
            })
        } else {
            None
        }
    }
}
