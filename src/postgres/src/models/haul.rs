use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use fiskeridir_rs::{Gear, GearGroup, SpeciesGroup, VesselLengthGroup, WhaleGender};
use kyogre_core::{CatchLocationId, HaulId};
use serde::Deserialize;

use crate::{error::PostgresError, queries::decimal_to_float};

use super::{HaulOceanClimate, HaulWeather};

#[derive(Debug)]
pub struct FishingSpotTrainingData {
    pub haul_id: i64,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub species: SpeciesGroup,
    pub week: i32,
    pub weight: f64,
}

#[derive(Deserialize)]
pub struct Haul {
    pub haul_id: i64,
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    pub catch_location_start: Option<String>,
    pub catch_locations: Option<Vec<String>>,
    pub ocean_depth_end: i32,
    pub ocean_depth_start: i32,
    pub quota_type_id: i32,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
    pub start_latitude: BigDecimal,
    pub start_longitude: BigDecimal,
    pub stop_latitude: BigDecimal,
    pub stop_longitude: BigDecimal,
    pub total_living_weight: i64,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: BigDecimal,
    pub vessel_length_group: VesselLengthGroup,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub wind_speed_10m: Option<BigDecimal>,
    pub wind_direction_10m: Option<BigDecimal>,
    pub air_temperature_2m: Option<BigDecimal>,
    pub relative_humidity_2m: Option<BigDecimal>,
    pub air_pressure_at_sea_level: Option<BigDecimal>,
    pub precipitation_amount: Option<BigDecimal>,
    pub cloud_area_fraction: Option<BigDecimal>,
    pub water_speed: Option<BigDecimal>,
    pub water_direction: Option<BigDecimal>,
    pub salinity: Option<BigDecimal>,
    pub water_temperature: Option<BigDecimal>,
    pub ocean_climate_depth: Option<BigDecimal>,
    pub sea_floor_depth: Option<BigDecimal>,
    pub catches: String,
    pub whale_catches: String,
}

#[derive(Deserialize)]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fao_id: String,
    pub species_fiskeridir_id: i32,
    pub species_group_id: i32,
    pub species_main_group_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct WhaleCatch {
    pub blubber_measure_a: Option<i32>,
    pub blubber_measure_b: Option<i32>,
    pub blubber_measure_c: Option<i32>,
    pub circumference: Option<i32>,
    pub fetus_length: Option<i32>,
    pub gender_id: Option<WhaleGender>,
    pub grenade_number: String,
    pub individual_number: Option<i32>,
    pub length: Option<i32>,
}

pub struct HaulMessage {
    pub haul_id: i64,
    pub message_id: i64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
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
            catch_locations: v
                .catch_locations
                .map(|c| c.into_iter().map(CatchLocationId::try_from).collect())
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
            total_living_weight: v.total_living_weight,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: decimal_to_float(v.vessel_length)
                .change_context(PostgresError::DataConversion)?,
            vessel_length_group: v.vessel_length_group,
            vessel_name: v.vessel_name,
            vessel_name_ers: v.vessel_name_ers,
            weather: HaulWeather {
                wind_speed_10m: v.wind_speed_10m,
                wind_direction_10m: v.wind_direction_10m,
                air_temperature_2m: v.air_temperature_2m,
                relative_humidity_2m: v.relative_humidity_2m,
                air_pressure_at_sea_level: v.air_pressure_at_sea_level,
                precipitation_amount: v.precipitation_amount,
                cloud_area_fraction: v.cloud_area_fraction,
            }
            .try_into()?,
            ocean_climate: HaulOceanClimate {
                water_speed: v.water_speed,
                water_direction: v.water_direction,
                salinity: v.salinity,
                water_temperature: v.water_temperature,
                ocean_climate_depth: v.ocean_climate_depth,
                sea_floor_depth: v.sea_floor_depth,
            }
            .try_into()?,
            catches: serde_json::from_str::<Vec<HaulCatch>>(&v.catches)
                .change_context(PostgresError::DataConversion)?
                .into_iter()
                .map(kyogre_core::HaulCatch::try_from)
                .collect::<Result<_, _>>()?,
            whale_catches: serde_json::from_str::<Vec<WhaleCatch>>(&v.whale_catches)
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
            gender_id: v.gender_id,
            grenade_number: v.grenade_number,
            individual_number: v.individual_number,
            length: v.length,
        })
    }
}

impl TryFrom<HaulMessage> for kyogre_core::HaulMessage {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            haul_id: HaulId(v.haul_id),
            start_timestamp: v.start_timestamp,
            stop_timestamp: v.stop_timestamp,
        })
    }
}

impl TryFrom<FishingSpotTrainingData> for kyogre_core::FishingSpotTrainingData {
    type Error = Report<PostgresError>;

    fn try_from(v: FishingSpotTrainingData) -> Result<Self, Self::Error> {
        Ok(Self {
            haul_id: v.haul_id,
            latitude: decimal_to_float(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: decimal_to_float(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            species: v.species,
            week: v.week,
            weight: v.weight,
        })
    }
}
