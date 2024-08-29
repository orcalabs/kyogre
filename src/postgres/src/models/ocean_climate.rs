use chrono::{DateTime, Utc};
use kyogre_core::WeatherLocationId;
use unnest_insert::UnnestInsert;

use crate::error::Error;

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "ocean_climate",
    conflict = "timestamp,depth,weather_location_id"
)]
pub struct NewOceanClimate {
    pub timestamp: DateTime<Utc>,
    pub depth: i32,
    pub latitude: f64,
    pub longitude: f64,
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub upward_sea_velocity: Option<f64>,
    pub wind_speed: Option<f64>,
    pub wind_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub temperature: Option<f64>,
    pub sea_floor_depth: f64,
}

#[derive(Debug)]
pub struct OceanClimate {
    pub timestamp: DateTime<Utc>,
    pub depth: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub upward_sea_velocity: Option<f64>,
    pub wind_speed: Option<f64>,
    pub wind_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub temperature: Option<f64>,
    pub sea_floor_depth: f64,
    pub weather_location_id: i32,
}

#[derive(Debug)]
pub struct HaulOceanClimate {
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub water_temperature: Option<f64>,
    pub ocean_climate_depth: Option<f64>,
    pub sea_floor_depth: Option<f64>,
}

impl TryFrom<kyogre_core::NewOceanClimate> for NewOceanClimate {
    type Error = Error;

    fn try_from(v: kyogre_core::NewOceanClimate) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            depth: v.depth,
            latitude: v.latitude,
            longitude: v.longitude,
            water_speed: v.water_speed,
            water_direction: v.water_direction,
            upward_sea_velocity: v.upward_sea_velocity,
            wind_speed: v.wind_speed,
            wind_direction: v.wind_direction,
            salinity: v.salinity,
            temperature: v.temperature,
            sea_floor_depth: v.sea_floor_depth,
        })
    }
}

impl TryFrom<OceanClimate> for kyogre_core::OceanClimate {
    type Error = Error;

    fn try_from(v: OceanClimate) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            depth: v.depth,
            latitude: v.latitude,
            longitude: v.longitude,
            water_speed: v.water_speed,
            water_direction: v.water_direction,
            upward_sea_velocity: v.upward_sea_velocity,
            wind_speed: v.wind_speed,
            wind_direction: v.wind_direction,
            salinity: v.salinity,
            temperature: v.temperature,
            sea_floor_depth: v.sea_floor_depth,
            weather_location_id: WeatherLocationId(v.weather_location_id),
        })
    }
}

impl TryFrom<HaulOceanClimate> for kyogre_core::HaulOceanClimate {
    type Error = Error;

    fn try_from(v: HaulOceanClimate) -> Result<Self, Self::Error> {
        Ok(Self {
            water_speed: v.water_speed,
            water_direction: v.water_direction,
            salinity: v.salinity,
            water_temperature: v.water_temperature,
            ocean_climate_depth: v.ocean_climate_depth,
            sea_floor_depth: v.sea_floor_depth,
        })
    }
}
