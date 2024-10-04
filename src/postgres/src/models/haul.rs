use chrono::{DateTime, Utc};
use fiskeridir_rs::{Gear, GearGroup, VesselLengthGroup};
use kyogre_core::{
    CatchLocationId, FiskeridirVesselId, HaulCatch, HaulId, HaulOceanClimate, HaulWeather,
    WhaleCatch,
};
use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Deserialize)]
pub struct Haul {
    pub haul_id: HaulId,
    pub ers_activity_id: String,
    pub duration: i32,
    pub haul_distance: Option<i32>,
    pub catch_location_start: Option<CatchLocationId>,
    pub catch_locations: Option<Vec<CatchLocationId>>,
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
    pub fiskeridir_vessel_id: Option<FiskeridirVesselId>,
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

impl TryFrom<Haul> for kyogre_core::Haul {
    type Error = Error;

    fn try_from(v: Haul) -> Result<Self> {
        Ok(Self {
            haul_id: v.haul_id,
            ers_activity_id: v.ers_activity_id,
            duration: v.duration,
            haul_distance: v.haul_distance,
            catch_location_start: v.catch_location_start,
            catch_locations: v.catch_locations,
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
            catches: serde_json::from_str::<Vec<HaulCatch>>(&v.catches)?,
            whale_catches: serde_json::from_str::<Vec<WhaleCatch>>(&v.whale_catches)?,
            cache_version: v.cache_version,
        })
    }
}
