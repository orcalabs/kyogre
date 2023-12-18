use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{angle_between_vectors, length_of_vector};

#[derive(Debug, Clone, Deserialize)]
pub struct Weather {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub x_wind_10m: Option<f64>,
    pub y_wind_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub land_area_fraction: f64,
    pub cloud_area_fraction: Option<f64>,
}

impl Weather {
    pub fn to_core_weather(v: Weather, timestamp: DateTime<Utc>) -> kyogre_core::NewWeather {
        let (wind_speed_10m, wind_direction_10m) =
            if v.wind_speed_10m.is_some() && v.wind_direction_10m.is_some() {
                (v.wind_speed_10m, v.wind_direction_10m)
            } else if let (Some(x), Some(y)) = (v.x_wind_10m, v.y_wind_10m) {
                (
                    Some(length_of_vector((x, y))),
                    Some(angle_between_vectors((1., 0.), (x, y))),
                )
            } else {
                (None, None)
            };

        kyogre_core::NewWeather {
            timestamp,
            latitude: v.latitude,
            longitude: v.longitude,
            altitude: v.altitude,
            wind_speed_10m: wind_speed_10m.into(),
            wind_direction_10m,
            land_area_fraction: v.land_area_fraction,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level.into(),
            air_temperature_2m: v.air_temperature_2m.into(),
            cloud_area_fraction: v.cloud_area_fraction.into(),
            precipitation_amount: v.precipitation_amount.into(),
            relative_humidity_2m: v.relative_humidity_2m.into(),
        }
    }
}
