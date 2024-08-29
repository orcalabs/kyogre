use chrono::{DateTime, NaiveDate, Utc};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::WeatherLocationId;
use unnest_insert::UnnestInsert;

use crate::error::{Error, MissingValueSnafu};

#[derive(UnnestInsert)]
#[unnest_insert(table_name = "daily_weather_dirty", conflict = "date")]
pub struct NewWeatherDailyDirty {
    pub date: NaiveDate,
}

#[derive(UnnestInsert)]
#[unnest_insert(table_name = "weather", conflict = "timestamp,weather_location_id")]
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

#[derive(Debug)]
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
    pub weather_location_id: i32,
}

#[derive(Debug)]
pub struct WeatherLocation {
    pub weather_location_id: i32,
    pub polygon: wkb::Decode<Geometry<f64>>,
}

#[derive(Debug)]
pub struct HaulWeather {
    pub wind_speed_10m: Option<f64>,
    pub wind_direction_10m: Option<f64>,
    pub air_temperature_2m: Option<f64>,
    pub relative_humidity_2m: Option<f64>,
    pub air_pressure_at_sea_level: Option<f64>,
    pub precipitation_amount: Option<f64>,
    pub cloud_area_fraction: Option<f64>,
}

impl TryFrom<kyogre_core::NewWeather> for NewWeather {
    type Error = Error;

    fn try_from(v: kyogre_core::NewWeather) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            latitude: v.latitude,
            longitude: v.longitude,
            altitude: v.altitude,
            wind_speed_10m: v.wind_speed_10m.into_inner(),
            wind_direction_10m: v.wind_direction_10m,
            air_temperature_2m: v.air_temperature_2m.into_inner(),
            relative_humidity_2m: v.relative_humidity_2m.into_inner(),
            air_pressure_at_sea_level: v.air_pressure_at_sea_level.into_inner(),
            precipitation_amount: v.precipitation_amount.into_inner(),
            land_area_fraction: v.land_area_fraction,
            cloud_area_fraction: v.cloud_area_fraction.into_inner(),
        })
    }
}

impl TryFrom<Weather> for kyogre_core::Weather {
    type Error = Error;

    fn try_from(v: Weather) -> Result<Self, Self::Error> {
        Ok(Self {
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
            weather_location_id: WeatherLocationId(v.weather_location_id),
        })
    }
}

impl TryFrom<WeatherLocation> for kyogre_core::WeatherLocation {
    type Error = Error;

    fn try_from(v: WeatherLocation) -> Result<Self, Self::Error> {
        let geometry = v
            .polygon
            .geometry
            .ok_or_else(|| MissingValueSnafu.build())?;

        let polygon = match geometry {
            Geometry::Polygon(p) => p,
            _ => return MissingValueSnafu.fail(),
        };

        Ok(Self {
            id: WeatherLocationId(v.weather_location_id),
            polygon,
        })
    }
}
impl TryFrom<HaulWeather> for kyogre_core::HaulWeather {
    type Error = Error;

    fn try_from(v: HaulWeather) -> Result<Self, Self::Error> {
        Ok(Self {
            wind_speed_10m: v.wind_speed_10m,
            wind_direction_10m: v.wind_direction_10m,
            air_temperature_2m: v.air_temperature_2m,
            relative_humidity_2m: v.relative_humidity_2m,
            air_pressure_at_sea_level: v.air_pressure_at_sea_level,
            precipitation_amount: v.precipitation_amount,
            cloud_area_fraction: v.cloud_area_fraction,
        })
    }
}
