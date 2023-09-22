use chrono::{DateTime, Utc};
use geo::geometry::Polygon;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{HaulId, HaulOceanClimate, HaulWeatherStatus};

#[derive(Debug, Clone)]
pub struct NewWeather {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Weather {
    pub timestamp: DateTime<Utc>,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
    pub weather_location_id: WeatherLocationId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct WeatherLocationId(pub i32);

#[derive(Debug, Clone)]
pub struct WeatherLocation {
    pub id: WeatherLocationId,
    pub polygon: Polygon,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaulWeather {
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct HaulWeatherOutput {
    pub haul_id: HaulId,
    pub weather: Option<HaulWeather>,
    pub ocean_climate: Option<HaulOceanClimate>,
    pub status: HaulWeatherStatus,
}

pub(crate) static WEATHER_LOCATION_LATS_LONS: [(f64, f64); 20] = [
    (52.380, 1.8924),
    (52.352, 1.9485),
    (52.357, 2.0484),
    (52.363, 2.1499),
    (52.369, 2.2486),
    (52.376, 2.3474),
    (52.381, 2.4476),
    (52.387, 2.5453),
    (52.392, 2.6376),
    (52.399, 2.7079),
    (52.458, 1.8801),
    (52.448, 1.9519),
    (52.450, 2.0505),
    (52.450, 2.1503),
    (52.449, 2.2514),
    (52.450, 2.3504),
    (52.450, 2.4490),
    (52.449, 2.5493),
    (52.449, 2.6498),
    (52.450, 2.7498),
];

impl NewWeather {
    pub fn test_default(timestamp: DateTime<Utc>) -> Self {
        let mut rng = rand::thread_rng();
        let (latitude, longitude) =
            WEATHER_LOCATION_LATS_LONS[rng.gen::<usize>() % WEATHER_LOCATION_LATS_LONS.len()];
        let num = rng.gen::<u8>() as f64;

        Self {
            timestamp,
            latitude,
            longitude,
            altitude: num,
            wind_speed_10m: Some(num),
            wind_direction_10m: Some(num),
            air_temperature_2m: Some(num),
            relative_humidity_2m: Some(num),
            air_pressure_at_sea_level: Some(num),
            precipitation_amount: Some(num),
            land_area_fraction: 0.,
            cloud_area_fraction: Some(0.),
        }
    }
}

impl From<&NewWeather> for Weather {
    fn from(v: &NewWeather) -> Self {
        Self {
            timestamp: v.timestamp,
            latitude: v.latitude,
            longitude: v.longitude,
            altitude: v.altitude,
            wind_speed_10m: v.wind_speed_10m,
            wind_direction_10m: v.wind_direction_10m,
            air_temperature_2m: v.air_temperature_2m,
            relative_humidity_2m: v.relative_humidity_2m,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level,
            precipitation_amount: v.precipitation_amount,
            land_area_fraction: v.land_area_fraction,
            cloud_area_fraction: v.cloud_area_fraction,
            weather_location_id: WeatherLocationId::from_lat_lon(v.latitude, v.longitude),
        }
    }
}

impl WeatherLocationId {
    pub fn from_lat_lon(lat: f64, lon: f64) -> Self {
        Self(
            (((lat / 0.1).floor() as i32 + 1_000) * 100_000) + ((lon / 0.1).floor() as i32 + 1_000),
        )
    }
}

impl PartialEq for WeatherLocation {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for WeatherLocation {}

impl std::hash::Hash for WeatherLocation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
