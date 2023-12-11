use chrono::{DateTime, NaiveDate, Utc};
use geo::geometry::Polygon;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{CatchLocationId, HaulId, HaulOceanClimate, HaulWeatherStatus};

#[derive(Copy, Clone, Debug)]
pub enum WeatherData {
    Require,
    Optional,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CatchLocationWeather {
    #[serde(skip_serializing)]
    pub id: CatchLocationId,
    pub date: NaiveDate,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: f64,
    pub air_temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub air_pressure_at_sea_level: f64,
    pub precipitation_amount: f64,
    pub cloud_area_fraction: f64,
}

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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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

pub static WEATHER_LOCATION_LATS_LONS: [(f64, f64, i64); 21] = [
    (71.35, 35.95, 171301359),
    (71.05, 28.05, 171001280),
    (71.05, 27.95, 171001279),
    (71.05, 25.95, 171001259),
    (71.04, 25.85, 171001258),
    (71.05, 25.75, 171001257),
    (71.04, 25.65, 171001256),
    (71.05, 25.55, 171001255),
    (70.95, 28.35, 170901283),
    (70.95, 28.24, 170901282),
    (70.95, 28.15, 170901281),
    (70.95, 28.05, 170901280),
    (70.95, 27.95, 170901279),
    (70.95, 27.84, 170901278),
    (70.95, 27.55, 170901275),
    (70.95, 27.44, 170901274),
    (70.95, 27.34, 170901273),
    (70.95, 26.65, 170901266),
    (70.95, 26.55, 170901265),
    (70.95, 25.55, 170901255),
    (70.95, 25.45, 170901254),
];

impl NewWeather {
    pub fn test_default(timestamp: DateTime<Utc>) -> Self {
        let mut rng = rand::thread_rng();
        let (latitude, longitude, _) = WEATHER_LOCATION_LATS_LONS[0];
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
