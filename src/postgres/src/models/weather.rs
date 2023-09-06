use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    queries::{decimal_to_float, float_to_decimal, opt_decimal_to_float, opt_float_to_decimal},
};

#[derive(UnnestInsert)]
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

#[derive(Debug)]
pub struct HaulWeather {
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

impl TryFrom<kyogre_core::NewWeather> for NewWeather {
    type Error = Report<PostgresError>;

    fn try_from(v: kyogre_core::NewWeather) -> Result<Self, Self::Error> {
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
            weather_location_id: v.weather_location_id,
        })
    }
}

impl TryFrom<HaulWeather> for kyogre_core::HaulWeather {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulWeather) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}
