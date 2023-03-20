use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Utc};
use error_stack::{IntoReport, Report, ResultExt};
use kyogre_core::{CatchLocationId, HaulId, WhaleGender};
use serde::Deserialize;

use crate::{
    error::{PostgresError, WhaleGenderError},
    queries::decimal_to_float,
};

#[derive(Deserialize)]
pub struct Haul {
    pub haul_id: String,
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    pub catch_location_start: Option<String>,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    pub start_latitude: BigDecimal,
    pub start_longitude: BigDecimal,
    pub stop_latitude: BigDecimal,
    pub stop_longitude: BigDecimal,
    pub gear_fiskeridir_id: Option<i32>,
    pub gear_group_id: Option<i32>,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: BigDecimal,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub catches: String,
    pub whale_catches: String,
}

#[derive(Deserialize)]
pub struct HaulCatch {
    pub main_species_fao_id: String,
    pub main_species_fiskeridir_id: Option<i32>,
    pub living_weight: i32,
    pub species_fao_id: String,
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
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}

#[derive(Debug)]
pub struct HaulsGrid {
    pub grid: String,
    pub max_weight: i64,
    pub min_weight: i64,
}

impl TryFrom<Haul> for kyogre_core::Haul {
    type Error = Report<PostgresError>;

    fn try_from(v: Haul) -> Result<Self, Self::Error> {
        Ok(Self {
            haul_id: HaulId(v.haul_id),
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_location_start: v
                .catch_location_start
                .map(CatchLocationId::try_from)
                .transpose()
                .change_context(PostgresError::DataConversion)?,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_latitude: decimal_to_float(v.start_latitude)
                .change_context(PostgresError::DataConversion)?,
            start_longitude: decimal_to_float(v.start_longitude)
                .change_context(PostgresError::DataConversion)?,
            start_timestamp: v.start_timestamp,
            stop_latitude: decimal_to_float(v.stop_latitude)
                .change_context(PostgresError::DataConversion)?,
            stop_longitude: decimal_to_float(v.stop_longitude)
                .change_context(PostgresError::DataConversion)?,
            stop_timestamp: v.stop_timestamp,
            gear_fiskeridir_id: v.gear_fiskeridir_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: decimal_to_float(v.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            catches: serde_json::from_str::<Vec<HaulCatch>>(&v.catches)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::HaulCatch::try_from)
                .collect::<Result<_, _>>()?,
            whale_catches: serde_json::from_str::<Vec<WhaleCatch>>(&v.whale_catches)
                .into_report()
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::WhaleCatch::try_from)
                .collect::<Result<_, _>>()?,
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

impl TryFrom<HaulsGrid> for kyogre_core::HaulsGrid {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulsGrid) -> Result<Self, Self::Error> {
        Ok(Self {
            grid: serde_json::from_str(&v.grid)
                .into_report()
                .change_context(PostgresError::DataConversion)
                .attach_printable_lazy(|| format!("could not serialize grid: {}", v.grid))?,
            max_weight: v.max_weight,
            min_weight: v.min_weight,
        })
    }
}
