use error_stack::{report, Report, ResultExt};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::CatchLocationId;

use crate::error::PostgresError;

pub struct CatchLocation {
    pub catch_location_id: String,
    pub polygon: wkb::Decode<Geometry<f64>>,
    pub latitude: f64,
    pub longitude: f64,
    pub weather_location_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct CatchLocationWeather {
    pub catch_main_area_id: i32,
    pub catch_area_id: i32,
    pub wind_speed_10m: f64,
    pub wind_direction_10m: f64,
    pub air_temperature_2m: f64,
    pub relative_humidity_2m: f64,
    pub air_pressure_at_sea_level: f64,
    pub precipitation_amount: f64,
    pub cloud_area_fraction: f64,
}

impl TryFrom<CatchLocation> for kyogre_core::CatchLocation {
    type Error = Report<PostgresError>;

    fn try_from(v: CatchLocation) -> Result<Self, Self::Error> {
        let geometry = v
            .polygon
            .geometry
            .ok_or_else(|| report!(PostgresError::DataConversion))?;

        let polygon = match geometry {
            Geometry::Polygon(p) => p,
            _ => return Err(report!(PostgresError::DataConversion)),
        };

        Ok(Self {
            id: CatchLocationId::try_from(v.catch_location_id)
                .change_context(PostgresError::DataConversion)?,
            polygon,
            latitude: v.latitude,
            longitude: v.longitude,
            weather_location_ids: v.weather_location_ids,
        })
    }
}

impl From<CatchLocationWeather> for kyogre_core::CatchLocationWeather {
    fn from(value: CatchLocationWeather) -> Self {
        Self {
            wind_speed_10m: value.wind_speed_10m,
            wind_direction_10m: value.wind_direction_10m,
            air_temperature_2m: value.air_temperature_2m,
            relative_humidity_2m: value.relative_humidity_2m,
            air_pressure_at_sea_level: value.air_pressure_at_sea_level,
            precipitation_amount: value.precipitation_amount,
            cloud_area_fraction: value.cloud_area_fraction,
            id: CatchLocationId::new(value.catch_main_area_id, value.catch_area_id),
        }
    }
}
