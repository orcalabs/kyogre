use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use error_stack::{Report, ResultExt};
use kyogre_core::WeatherLocationId;
use unnest_insert::UnnestInsert;

use crate::{
    error::PostgresError,
    queries::{decimal_to_float, float_to_decimal, opt_decimal_to_float, opt_float_to_decimal},
};

#[derive(UnnestInsert)]
#[unnest_insert(
    table_name = "ocean_climate",
    conflict = "timestamp,depth,weather_location_id"
)]
pub struct NewOceanClimate {
    pub timestamp: DateTime<Utc>,
    pub depth: i32,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub water_speed: Option<BigDecimal>,
    pub water_direction: Option<BigDecimal>,
    pub upward_sea_velocity: Option<BigDecimal>,
    pub wind_speed: Option<BigDecimal>,
    pub wind_direction: Option<BigDecimal>,
    pub salinity: Option<BigDecimal>,
    pub temperature: Option<BigDecimal>,
    pub sea_floor_depth: BigDecimal,
}

#[derive(Debug)]
pub struct OceanClimate {
    pub timestamp: DateTime<Utc>,
    pub depth: BigDecimal,
    pub latitude: BigDecimal,
    pub longitude: BigDecimal,
    pub water_speed: Option<BigDecimal>,
    pub water_direction: Option<BigDecimal>,
    pub upward_sea_velocity: Option<BigDecimal>,
    pub wind_speed: Option<BigDecimal>,
    pub wind_direction: Option<BigDecimal>,
    pub salinity: Option<BigDecimal>,
    pub temperature: Option<BigDecimal>,
    pub sea_floor_depth: BigDecimal,
    pub weather_location_id: i32,
}

#[derive(Debug)]
pub struct HaulOceanClimate {
    pub water_speed: Option<BigDecimal>,
    pub water_direction: Option<BigDecimal>,
    pub salinity: Option<BigDecimal>,
    pub water_temperature: Option<BigDecimal>,
    pub ocean_climate_depth: Option<BigDecimal>,
    pub sea_floor_depth: Option<BigDecimal>,
}

impl TryFrom<kyogre_core::NewOceanClimate> for NewOceanClimate {
    type Error = Report<PostgresError>;

    fn try_from(v: kyogre_core::NewOceanClimate) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            depth: v.depth,
            latitude: float_to_decimal(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: float_to_decimal(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            water_speed: opt_float_to_decimal(v.water_speed)
                .change_context(PostgresError::DataConversion)?,
            water_direction: opt_float_to_decimal(v.water_direction)
                .change_context(PostgresError::DataConversion)?,
            upward_sea_velocity: opt_float_to_decimal(v.upward_sea_velocity)
                .change_context(PostgresError::DataConversion)?,
            wind_speed: opt_float_to_decimal(v.wind_speed)
                .change_context(PostgresError::DataConversion)?,
            wind_direction: opt_float_to_decimal(v.wind_direction)
                .change_context(PostgresError::DataConversion)?,
            salinity: opt_float_to_decimal(v.salinity)
                .change_context(PostgresError::DataConversion)?,
            temperature: opt_float_to_decimal(v.temperature)
                .change_context(PostgresError::DataConversion)?,
            sea_floor_depth: float_to_decimal(v.sea_floor_depth)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}

impl TryFrom<OceanClimate> for kyogre_core::OceanClimate {
    type Error = Report<PostgresError>;

    fn try_from(v: OceanClimate) -> Result<Self, Self::Error> {
        Ok(Self {
            timestamp: v.timestamp,
            depth: decimal_to_float(v.depth).change_context(PostgresError::DataConversion)?,
            latitude: decimal_to_float(v.latitude).change_context(PostgresError::DataConversion)?,
            longitude: decimal_to_float(v.longitude)
                .change_context(PostgresError::DataConversion)?,
            water_speed: opt_decimal_to_float(v.water_speed)
                .change_context(PostgresError::DataConversion)?,
            water_direction: opt_decimal_to_float(v.water_direction)
                .change_context(PostgresError::DataConversion)?,
            upward_sea_velocity: opt_decimal_to_float(v.upward_sea_velocity)
                .change_context(PostgresError::DataConversion)?,
            wind_speed: opt_decimal_to_float(v.wind_speed)
                .change_context(PostgresError::DataConversion)?,
            wind_direction: opt_decimal_to_float(v.wind_direction)
                .change_context(PostgresError::DataConversion)?,
            salinity: opt_decimal_to_float(v.salinity)
                .change_context(PostgresError::DataConversion)?,
            temperature: opt_decimal_to_float(v.temperature)
                .change_context(PostgresError::DataConversion)?,
            sea_floor_depth: decimal_to_float(v.sea_floor_depth)
                .change_context(PostgresError::DataConversion)?,
            weather_location_id: WeatherLocationId(v.weather_location_id),
        })
    }
}

impl TryFrom<HaulOceanClimate> for kyogre_core::HaulOceanClimate {
    type Error = Report<PostgresError>;

    fn try_from(v: HaulOceanClimate) -> Result<Self, Self::Error> {
        Ok(Self {
            water_speed: opt_decimal_to_float(v.water_speed)
                .change_context(PostgresError::DataConversion)?,
            water_direction: opt_decimal_to_float(v.water_direction)
                .change_context(PostgresError::DataConversion)?,
            salinity: opt_decimal_to_float(v.salinity)
                .change_context(PostgresError::DataConversion)?,
            water_temperature: opt_decimal_to_float(v.water_temperature)
                .change_context(PostgresError::DataConversion)?,
            ocean_climate_depth: opt_decimal_to_float(v.ocean_climate_depth)
                .change_context(PostgresError::DataConversion)?,
            sea_floor_depth: opt_decimal_to_float(v.sea_floor_depth)
                .change_context(PostgresError::DataConversion)?,
        })
    }
}
