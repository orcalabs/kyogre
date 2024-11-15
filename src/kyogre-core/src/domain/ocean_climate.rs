use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::WeatherLocationId;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
    pub weather_location_id: WeatherLocationId,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct HaulOceanClimate {
    pub water_speed: Option<f64>,
    pub water_direction: Option<f64>,
    pub salinity: Option<f64>,
    pub water_temperature: Option<f64>,
    pub ocean_climate_depth: Option<f64>,
    pub sea_floor_depth: Option<f64>,
}

impl From<&NewOceanClimate> for OceanClimate {
    fn from(v: &NewOceanClimate) -> Self {
        Self {
            timestamp: v.timestamp,
            latitude: v.latitude,
            longitude: v.longitude,
            depth: v.depth as f64,
            water_speed: v.water_speed,
            water_direction: v.water_direction,
            upward_sea_velocity: v.upward_sea_velocity,
            wind_speed: v.wind_speed,
            wind_direction: v.wind_direction,
            salinity: v.salinity,
            temperature: v.temperature,
            sea_floor_depth: v.sea_floor_depth,
            weather_location_id: WeatherLocationId::from_lat_lon(v.latitude, v.longitude),
        }
    }
}

#[cfg(feature = "test")]
mod test {
    use rand::Rng;

    use crate::WEATHER_LOCATION_LATS_LONS;

    use super::*;

    impl NewOceanClimate {
        pub fn test_default(timestamp: DateTime<Utc>) -> Self {
            let mut rng = rand::thread_rng();
            let (latitude, longitude, _) =
                WEATHER_LOCATION_LATS_LONS[rng.gen::<usize>() % WEATHER_LOCATION_LATS_LONS.len()];
            let num = rng.gen::<u8>() as f64;

            Self {
                timestamp,
                latitude,
                longitude,
                depth: 0,
                water_speed: Some(num),
                water_direction: Some(num),
                upward_sea_velocity: Some(num),
                wind_speed: Some(num),
                wind_direction: Some(num),
                salinity: Some(num),
                temperature: Some(num),
                sea_floor_depth: num,
            }
        }
    }
}
