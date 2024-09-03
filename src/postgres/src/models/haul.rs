use chrono::{DateTime, NaiveDate, Utc};
use fiskeridir_rs::{
    Gear, GearGroup, SpeciesGroup, SpeciesMainGroup, VesselLengthGroup, WhaleGender,
};
use kyogre_core::{CatchLocationId, HaulId, HaulOceanClimate, HaulWeather};
use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug)]
pub struct FishingSpotTrainingData {
    pub haul_id: HaulId,
    pub latitude: f64,
    pub longitude: f64,
    pub species: SpeciesGroup,
    pub catch_location: String,
    pub date: NaiveDate,
}

#[derive(Deserialize)]
pub struct Haul {
    pub haul_id: HaulId,
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
    pub start_latitude: f64,
    pub start_longitude: f64,
    pub stop_latitude: f64,
    pub stop_longitude: f64,
    pub total_living_weight: i64,
    pub gear_id: Gear,
    pub gear_group_id: GearGroup,
    pub fiskeridir_vessel_id: Option<i64>,
    pub vessel_call_sign: Option<String>,
    pub vessel_call_sign_ers: String,
    pub vessel_length: f64,
    pub vessel_length_group: VesselLengthGroup,
    pub vessel_name: Option<String>,
    pub vessel_name_ers: Option<String>,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub water_temperature: Option<f64>,
    pub ocean_climate_depth: Option<f64>,
    pub sea_floor_depth: Option<f64>,
    pub catches: String,
    pub whale_catches: String,
    pub cache_version: i64,
}

#[derive(Deserialize)]
pub struct HaulCatch {
    pub living_weight: i32,
    pub species_fao_id: String,
    pub species_fiskeridir_id: i32,
    pub species_group_id: SpeciesGroup,
    pub species_main_group_id: Option<SpeciesMainGroup>,
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
    pub haul_id: HaulId,
    pub message_id: i64,
    pub start_timestamp: DateTime<Utc>,
    pub stop_timestamp: DateTime<Utc>,
}

impl TryFrom<Haul> for kyogre_core::Haul {
    type Error = Error;

    fn try_from(v: Haul) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            haul_id: v.haul_id,
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_location_start: v
                .catch_location_start
                .map(CatchLocationId::try_from)
                .transpose()?,
            catch_locations: v
                .catch_locations
                .map(|c| c.into_iter().map(CatchLocationId::try_from).collect())
                .transpose()?,
            ocean_depth_end: v.ocean_depth_end,
            ocean_depth_start: v.ocean_depth_start,
            quota_type_id: v.quota_type_id,
            start_latitude: v.start_latitude,
            start_longitude: v.start_longitude,
            start_timestamp: v.start_timestamp,
            stop_latitude: v.stop_latitude,
            stop_longitude: v.stop_longitude,
            stop_timestamp: v.stop_timestamp,
            total_living_weight: v.total_living_weight,
            gear_id: v.gear_id,
            gear_group_id: v.gear_group_id,
            fiskeridir_vessel_id: v.fiskeridir_vessel_id,
            vessel_call_sign: v.vessel_call_sign,
            vessel_call_sign_ers: v.vessel_call_sign_ers,
            vessel_length: v.vessel_length,
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
            },
            ocean_climate: HaulOceanClimate {
                water_speed: v.water_speed,
                water_direction: v.water_direction,
                salinity: v.salinity,
                water_temperature: v.water_temperature,
                ocean_climate_depth: v.ocean_climate_depth,
                sea_floor_depth: v.sea_floor_depth,
            },
            catches: serde_json::from_str::<Vec<HaulCatch>>(&v.catches)?
                .into_iter()
                .map(kyogre_core::HaulCatch::try_from)
                .collect::<Result<_>>()?,
            whale_catches: serde_json::from_str::<Vec<WhaleCatch>>(&v.whale_catches)?
                .into_iter()
                .map(kyogre_core::WhaleCatch::try_from)
                .collect::<Result<_>>()?,
            cache_version: v.cache_version,
        })
    }
}

impl TryFrom<HaulCatch> for kyogre_core::HaulCatch {
    type Error = Error;

    fn try_from(v: HaulCatch) -> std::result::Result<Self, Self::Error> {
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
    type Error = Error;

    fn try_from(v: WhaleCatch) -> std::result::Result<Self, Self::Error> {
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
    type Error = Error;

    fn try_from(v: HaulMessage) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            haul_id: v.haul_id,
            start_timestamp: v.start_timestamp,
            stop_timestamp: v.stop_timestamp,
        })
    }
}

impl TryFrom<FishingSpotTrainingData> for kyogre_core::FishingSpotTrainingData {
    type Error = Error;

    fn try_from(v: FishingSpotTrainingData) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            haul_id: v.haul_id,
            latitude: v.latitude,
            longitude: v.longitude,
            species: v.species,
            catch_location_id: CatchLocationId::try_from(v.catch_location)?,
            date: v.date,
        })
    }
}
