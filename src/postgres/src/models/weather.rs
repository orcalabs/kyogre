use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{bail, report, Report, ResultExt};
use geo_types::geometry::Geometry;
use geozero::wkb;
use kyogre_core::{WeatherFftEntry, WeatherLocationId};
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    fft::FftEntry,
    queries::{decimal_to_float, float_to_decimal, opt_decimal_to_float, opt_float_to_decimal},
};

#[derive(Debug, Clone, UnnestInsert)]
#[unnest_insert(table_name = "weather", conflict = "timestamp,weather_location_id")]
pub struct NewWeather {
    pub timestamp: DateTime<Utc>,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub altitude: BigDecimal,
    pub wind_speed_10m: Option<BigDecimal>,
    pub wind_direction_10m: Option<BigDecimal>,
    pub air_temperature_2m: Option<BigDecimal>,
    pub relative_humidity_2m: Option<BigDecimal>,
    pub air_pressure_at_sea_level: Option<BigDecimal>,
    pub precipitation_amount: Option<BigDecimal>,
    pub land_area_fraction: BigDecimal,
    pub cloud_area_fraction: Option<BigDecimal>,
}

#[derive(Debug)]
pub struct Weather {
    pub timestamp: DateTime<Utc>,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub altitude: BigDecimal,
    pub wind_speed_10m: Option<BigDecimal>,
    pub wind_direction_10m: Option<BigDecimal>,
    pub air_temperature_2m: Option<BigDecimal>,
    pub relative_humidity_2m: Option<BigDecimal>,
    pub air_pressure_at_sea_level: Option<BigDecimal>,
    pub precipitation_amount: Option<BigDecimal>,
    pub land_area_fraction: BigDecimal,
    pub cloud_area_fraction: Option<BigDecimal>,
    pub weather_location_id: i32,
}

#[derive(Debug, Clone)]
pub struct WeatherFft {
    pub timestamp: DateTime<Utc>,
    pub wind_speed_10m: Vec<FftEntry>,
    pub air_temperature_2m: Vec<FftEntry>,
    pub relative_humidity_2m: Vec<FftEntry>,
    pub air_pressure_at_sea_level: Vec<FftEntry>,
    pub precipitation_amount: Vec<FftEntry>,
}

#[derive(Debug)]
pub struct WeatherLocation {
    pub weather_location_id: i32,
    pub polygon: wkb::Decode<Geometry<f64>>,
}

#[derive(Debug)]
pub struct HaulWeather {
    pub wind_speed_10m: Option<BigDecimal>,
    pub wind_direction_10m: Option<BigDecimal>,
    pub air_temperature_2m: Option<BigDecimal>,
    pub relative_humidity_2m: Option<BigDecimal>,
    pub air_pressure_at_sea_level: Option<BigDecimal>,
    pub precipitation_amount: Option<BigDecimal>,
    pub cloud_area_fraction: Option<BigDecimal>,
}

impl TryFrom<&kyogre_core::NewWeather> for NewWeather {
    type Error = Report<PostgresError>;

    fn try_from(v: &kyogre_core::NewWeather) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            latitude: float_to_decimal(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: float_to_decimal(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            altitude: float_to_decimal(v.altitude).change_context(PostgresError::DataConversion)?,
            wind_speed_10m: opt_float_to_decimal(v.wind_speed_10m)
                .change_context(PostgresError::DataConversion)?,
            wind_direction_10m: opt_float_to_decimal(v.wind_direction_10m)
                .change_context(PostgresError::DataConversion)?,
            air_temperature_2m: opt_float_to_decimal(v.air_temperature_2m)
                .change_context(PostgresError::DataConversion)?,
            relative_humidity_2m: opt_float_to_decimal(v.relative_humidity_2m)
                .change_context(PostgresError::DataConversion)?,
            air_pressure_at_sea_level: opt_float_to_decimal(v.air_pressure_at_sea_level)
                .change_context(PostgresError::DataConversion)?,
            precipitation_amount: opt_float_to_decimal(v.precipitation_amount)
                .change_context(PostgresError::DataConversion)?,
            land_area_fraction: float_to_decimal(v.land_area_fraction)
                .change_context(PostgresError::DataConversion)?,
            cloud_area_fraction: opt_float_to_decimal(v.cloud_area_fraction)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}

impl TryFrom<Weather> for kyogre_core::Weather {
    type Error = Report<PostgresError>;

    fn try_from(v: Weather) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            latitude: decimal_to_float(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: decimal_to_float(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            altitude: decimal_to_float(v.altitude).change_context(PostgresError::DataConversion)?,
            wind_speed_10m: opt_decimal_to_float(v.wind_speed_10m)
                .change_context(PostgresError::DataConversion)?,
            wind_direction_10m: opt_decimal_to_float(v.wind_direction_10m)
                .change_context(PostgresError::DataConversion)?,
            air_temperature_2m: opt_decimal_to_float(v.air_temperature_2m)
                .change_context(PostgresError::DataConversion)?,
            relative_humidity_2m: opt_decimal_to_float(v.relative_humidity_2m)
                .change_context(PostgresError::DataConversion)?,
            air_pressure_at_sea_level: opt_decimal_to_float(v.air_pressure_at_sea_level)
                .change_context(PostgresError::DataConversion)?,
            precipitation_amount: opt_decimal_to_float(v.precipitation_amount)
                .change_context(PostgresError::DataConversion)?,
            land_area_fraction: decimal_to_float(v.land_area_fraction)
                .change_context(PostgresError::DataConversion)?,
            cloud_area_fraction: opt_decimal_to_float(v.cloud_area_fraction)
                .change_context(PostgresError::DataConversion)?,
            weather_location_id: WeatherLocationId(v.weather_location_id),
        })
    }
}

impl TryFrom<WeatherFft> for kyogre_core::WeatherFft {
    type Error = Report<PostgresError>;

    fn try_from(v: WeatherFft) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            wind_speed_10m: v
                .wind_speed_10m
                .into_iter()
                .map(WeatherFftEntry::from)
                .collect(),
            air_temperature_2m: v
                .air_temperature_2m
                .into_iter()
                .map(WeatherFftEntry::from)
                .collect(),
            relative_humidity_2m: v
                .relative_humidity_2m
                .into_iter()
                .map(WeatherFftEntry::from)
                .collect(),
            air_pressure_at_sea_level: v
                .air_pressure_at_sea_level
                .into_iter()
                .map(WeatherFftEntry::from)
                .collect(),
            precipitation_amount: v
                .precipitation_amount
                .into_iter()
                .map(WeatherFftEntry::from)
                .collect(),
        })
    }
}

impl TryFrom<WeatherLocation> for kyogre_core::WeatherLocation {
    type Error = Report<PostgresError>;

    fn try_from(v: WeatherLocation) -> Result<Self, Self::Error> {
        let geometry = v
            .polygon
            .geometry
            .ok_or_else(|| report!(PostgresError::DataConversion))?;

        let polygon = match geometry {
            Geometry::Polygon(p) => p,
            _ => bail!(PostgresError::DataConversion),
        };

        Ok(Self {
            id: WeatherLocationId(v.weather_location_id),
            polygon,
        })
    }
}
impl TryFrom<HaulWeather> for kyogre_core::HaulWeather {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulWeather) -> Result<Self, Self::Error> {
        Ok(Self {
            wind_speed_10m: opt_decimal_to_float(v.wind_speed_10m)
                .change_context(PostgresError::DataConversion)?,
            wind_direction_10m: opt_decimal_to_float(v.wind_direction_10m)
                .change_context(PostgresError::DataConversion)?,
            air_temperature_2m: opt_decimal_to_float(v.air_temperature_2m)
                .change_context(PostgresError::DataConversion)?,
            relative_humidity_2m: opt_decimal_to_float(v.relative_humidity_2m)
                .change_context(PostgresError::DataConversion)?,
            air_pressure_at_sea_level: opt_decimal_to_float(v.air_pressure_at_sea_level)
                .change_context(PostgresError::DataConversion)?,
            precipitation_amount: opt_decimal_to_float(v.precipitation_amount)
                .change_context(PostgresError::DataConversion)?,
            cloud_area_fraction: opt_decimal_to_float(v.cloud_area_fraction)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}
